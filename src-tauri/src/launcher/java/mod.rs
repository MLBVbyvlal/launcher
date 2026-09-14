//! Java runtime discovery and Adoptium provisioning.
use super::*;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

mod provision;

pub use provision::download_java_major;
use provision::extract_zip_all;
// ─── Java helpers ─────────────────────────────────────────────────────────────

pub fn folder_java_major(name: &str) -> Option<u32> {
    name.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .find(|&n| n >= 8)
}

pub fn find_java_exe_recursive(dir: &PathBuf, exe: &str) -> Option<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return None };
    let mut subdirs = vec![];
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.file_name().map(|n| n == exe).unwrap_or(false) {
            return Some(path);
        }
    }
    for sub in subdirs {
        if let Some(found) = find_java_exe_recursive(&sub, exe) {
            return Some(found);
        }
    }
    None
}

pub(super) fn find_java(root: &PathBuf, req: Option<&JavaVersionReq>) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "javaw.exe" } else { "java" };
    let req_major = req.map(|r| r.major_version);

    // Exact match for mlbv-managed JREs (prevents Java 25 being used when Java 21 is needed)
    let ver_exact  = |detected: Option<u32>| match (detected, req_major) {
        (Some(v), Some(r)) => v == r,
        (None, Some(_))    => false,
        (_, None)          => true,
    };
    // Compatible (>=) for system-installed Java we don't control
    let ver_compat = |detected: Option<u32>| match (detected, req_major) {
        (Some(v), Some(r)) => v >= r,
        (None, Some(_))    => false,
        (_, None)          => true,
    };

    // 1. mlbv's own downloaded JREs (shared/java/jre-{N}/) — exact version required
    let mlbv_java = root.join("java");
    if mlbv_java.exists() {
        if let Ok(entries) = fs::read_dir(&mlbv_java) {
            let mut dirs: Vec<_> = entries.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .collect();
            dirs.sort_by_key(|e| folder_java_major(&e.file_name().to_string_lossy()).unwrap_or(0));
            for entry in dirs.iter() {
                let ver = folder_java_major(&entry.file_name().to_string_lossy());
                if ver_exact(ver) {
                    if let Some(java) = find_java_exe_recursive(&entry.path(), exe) {
                        return Some(java);
                    }
                }
            }
        }
    }

    // 2. Minecraft's own bundled runtime
    if let Some(req) = req {
        let component = &req.component;
        let rt_base = root.join("runtime").join(component);
        let platforms: &[&str] = if cfg!(windows) {
            &["windows-x64", "windows-x86", "windows"]
        } else if cfg!(target_os = "macos") {
            &["mac-os", "mac-os-arm64"]
        } else {
            &["linux", "linux-i386"]
        };
        for platform in platforms {
            let java = rt_base.join(platform).join(component).join("bin").join(exe);
            if java.exists() { return Some(java); }
            let java_mac = rt_base.join(platform).join(component)
                .join("jre.bundle/Contents/Home/bin/java");
            if java_mac.exists() { return Some(java_mac); }
        }
    }

    // 3. JAVA_HOME — version from last path component
    if let Ok(home) = std::env::var("JAVA_HOME") {
        let home_path = PathBuf::from(&home);
        let p = home_path.join("bin").join(exe);
        if p.exists() {
            let ver = home_path.file_name()
                .and_then(|n| folder_java_major(&n.to_string_lossy()));
            if ver_compat(ver) { return Some(p); }
        }
    }

    // 4. Common install paths — pick lowest satisfying version
    #[cfg(windows)]
    {
        let bases = [
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\Microsoft",
            "C:\\Program Files\\Amazon Corretto",
            "C:\\Program Files\\Java",
            "C:\\Program Files (x86)\\Java",
        ];
        for base in &bases {
            if let Ok(rd) = std::fs::read_dir(base) {
                let mut dirs: Vec<_> = rd.flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .collect();
                dirs.sort_by_key(|e| folder_java_major(&e.file_name().to_string_lossy()).unwrap_or(0));
                for entry in dirs.iter() {
                    let ver = folder_java_major(&entry.file_name().to_string_lossy());
                    let p = entry.path().join("bin").join(exe);
                    if p.exists() && ver_compat(ver) { return Some(p); }
                }
            }
        }
    }

    // 4b. Same for Linux: distro packages live in /usr/lib/jvm, manual
    // installs usually land in /usr/java or /opt.
    #[cfg(target_os = "linux")]
    {
        let bases = ["/usr/lib/jvm", "/usr/java", "/opt", "/opt/java"];
        for base in &bases {
            if let Ok(rd) = std::fs::read_dir(base) {
                let mut dirs: Vec<_> = rd.flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .collect();
                dirs.sort_by_key(|e| folder_java_major(&e.file_name().to_string_lossy()).unwrap_or(0));
                for entry in dirs.iter() {
                    let ver = folder_java_major(&entry.file_name().to_string_lossy());
                    let p = entry.path().join("bin").join(exe);
                    if p.exists() && ver_compat(ver) { return Some(p); }
                }
            }
        }
    }

    // 5. PATH fallback (any version if no requirement)
    if req_major.is_none()
        && std::process::Command::new(exe).arg("-version").output().is_ok()
    {
        return Some(PathBuf::from(exe));
    }

    None
}

pub async fn ensure_java(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    shared_dir: &PathBuf,
    req: Option<&JavaVersionReq>,
    java_override: Option<&str>,
    ctx: Option<&Ctx<'_>>,
) -> Result<PathBuf> {
    // Progress and the byte counter are attributed to the launching
    // instance when there is one (standalone callers pass None).
    let instance = ctx.map(|c| c.instance.as_str()).unwrap_or("");
    let progress = |stage: &str, pct: f32, msg: &str| progress(app, instance, stage, pct, msg);
    // Explicit user override (Settings → Java) wins over auto-detection.
    // A dangling path falls back to auto-detect instead of breaking the
    // launch — the user may have uninstalled that JDK.
    if let Some(custom) = java_override.map(str::trim).filter(|s| !s.is_empty()) {
        let p = PathBuf::from(custom);
        if p.is_file() {
            progress("launch", 88.0, &format!("Java: using custom executable {}", p.display()));
            return Ok(p);
        }
        progress("launch", 88.0, &format!("Java: custom path not found ({custom}), falling back to auto-detect…"));
    }

    let major = req.map(|r| r.major_version).unwrap_or(21);
    let exe   = if cfg!(windows) { "javaw.exe" } else { "java" };

    progress("launch", 88.0, &format!("Java {major}: scanning local installs (shared_dir={})…", shared_dir.display()));
    if let Some(java) = find_java(shared_dir, req).or_else(|| find_java(&mc_dir(), req)) {
        progress("launch", 88.5, &format!("Java {major}: found at {}", java.display()));
        return Ok(java);
    }

    let java_dir = shared_dir.join("java").join(format!("jre-{major}"));
    progress("launch", 88.5, &format!("Java {major}: not found locally, checking {}", java_dir.display()));

    // Already extracted from a previous download?
    if java_dir.exists() {
        progress("launch", 88.5, &format!("Java {major}: dir exists, scanning for {exe}…"));
        if let Some(found) = find_java_exe_recursive(&java_dir, exe) {
            return Ok(found);
        }
        progress("launch", 88.5, &format!("Java {major}: dir exists but no {exe} found inside"));
    }

    // Auto-download Eclipse Temurin JRE via Adoptium v3 feature_releases API
    // Note: /assets/latest/ is broken (404); /assets/feature_releases/ works and uses binaries[] array
    let os_str   = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch_str = match std::env::consts::ARCH { "x86_64" => "x64", "aarch64" => "aarch64", _ => "x86" };
    let assets_url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{major}/ga?architecture={arch_str}&heap_size=normal&image_type=jre&os={os_str}&vendor=eclipse&page_size=1"
    );
    progress("download", 88.0, &format!("Java {major}: querying Adoptium (os={os_str} arch={arch_str})…"));

    let api_resp = client
        .get(&assets_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot reach Adoptium API (adoptium.net)")?;
    let api_status = api_resp.status();
    let assets_json: serde_json::Value = api_resp.json().await
        .with_context(|| format!("Adoptium assets JSON parse failed (HTTP {api_status}, url={assets_url})"))?;

    progress("download", 88.2, &format!("Java {major}: Adoptium replied HTTP {api_status}, entries={}", assets_json.as_array().map(|a| a.len()).unwrap_or(0)));

    let direct_url = assets_json[0]["binaries"][0]["package"]["link"]
        .as_str()
        .ok_or_else(|| anyhow!(
            "Java {major} not found in Adoptium catalog (HTTP {api_status}, entries={}, url={assets_url}) — install manually: https://adoptium.net",
            assets_json.as_array().map(|a| a.len()).unwrap_or(0)
        ))?
        .to_string();

    progress("download", 88.3, &format!("Java {major}: got CDN url, downloading…"));

    let resp = client.get(&direct_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot download Java from Adoptium CDN")?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Java {major} download failed (HTTP {})",
            resp.status()
        ));
    }
    let total = resp.content_length().unwrap_or(0);
    // Stream straight to disk — a JRE archive is ~40–50 MB and does not need
    // to sit in RAM. Bytes are counted into the shared speed counter so the
    // live speed readout covers Java downloads too.
    fs::create_dir_all(&java_dir)?;
    let zip_path = java_dir.join("jre.zip");
    let fallback = AtomicU64::new(0);
    let dl_bytes: &AtomicU64 = ctx.map(|c| &c.ctl.bytes).unwrap_or(&fallback);
    {
        use std::io::Write;
        let mut file = std::io::BufWriter::new(fs::File::create(&zip_path)?);
        let mut downloaded: u64 = 0;
        let mut last_mb: u64 = 0;
        let mut resp = resp;
        while let Some(chunk) = resp.chunk().await.context("Java download interrupted")? {
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            dl_bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            let mb = downloaded / 1_048_576;
            if mb > last_mb {
                last_mb = mb;
                if total > 0 {
                    let tot = total / 1_048_576;
                    let pct = 88.0_f32 + (downloaded as f32 / total as f32) * 5.0;
                    progress("download", pct, &format!("Java {major}: {mb}/{tot} MB…"));
                } else {
                    progress("download", 89.0, &format!("Java {major}: {mb} MB…"));
                }
            }
        }
        file.flush()?;
    }

    progress("download", 93.5, &format!("Installing Java {major}…"));
    extract_zip_all(&zip_path, &java_dir)?;
    let _ = fs::remove_file(&zip_path);

    find_java_exe_recursive(&java_dir, exe)
        .ok_or_else(|| anyhow!("Java {major} installed but executable not found in extracted archive"))
}

pub fn scan_java_installs() -> Vec<(u32, String)> {
    let exe = if cfg!(windows) { "javaw.exe" } else { "java" };
    let mut found: Vec<(u32, String)> = vec![];

    // mlbv's own downloads
    let shared = shared_data_dir();
    let mlbv_java = shared.join("java");
    if let Ok(entries) = fs::read_dir(&mlbv_java) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
            if let Some(major) = folder_java_major(&entry.file_name().to_string_lossy()) {
                if find_java_exe_recursive(&entry.path(), exe).is_some() {
                    found.push((major, entry.path().to_string_lossy().into_owned()));
                }
            }
        }
    }

    // Common install paths
    #[cfg(windows)]
    {
        let bases = [
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\Microsoft",
            "C:\\Program Files\\Amazon Corretto",
            "C:\\Program Files\\Java",
            "C:\\Program Files (x86)\\Java",
        ];
        for base in &bases {
            if let Ok(rd) = std::fs::read_dir(base) {
                for entry in rd.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(major) = folder_java_major(&name) {
                        let p = entry.path().join("bin").join(exe);
                        if p.exists() && !found.iter().any(|(v, _)| *v == major) {
                            found.push((major, entry.path().to_string_lossy().into_owned()));
                        }
                    }
                }
            }
        }
    }

    // Same for Linux system installs.
    #[cfg(target_os = "linux")]
    {
        let bases = ["/usr/lib/jvm", "/usr/java", "/opt", "/opt/java"];
        for base in &bases {
            if let Ok(rd) = std::fs::read_dir(base) {
                for entry in rd.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(major) = folder_java_major(&name) {
                        let p = entry.path().join("bin").join(exe);
                        if p.exists() && !found.iter().any(|(v, _)| *v == major) {
                            found.push((major, entry.path().to_string_lossy().into_owned()));
                        }
                    }
                }
            }
        }
    }

    found.sort_by_key(|(v, _)| *v);
    found
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_java_major_reads_first_real_version() {
        assert_eq!(folder_java_major("jre-8"), Some(8));
        assert_eq!(folder_java_major("jre-21"), Some(21));
        assert_eq!(folder_java_major("jdk-17.0.9"), Some(17));
        assert_eq!(folder_java_major("temurin-8-jre"), Some(8));
        // Old-style "1.8" numbering still resolves to Java 8.
        assert_eq!(folder_java_major("jdk1.8.0_392"), Some(8));
        assert_eq!(folder_java_major("17"), Some(17));
        assert_eq!(folder_java_major("jdk-7"), None);
        assert_eq!(folder_java_major("java"), None);
        assert_eq!(folder_java_major(""), None);
    }
    #[test]
    fn find_java_exe_recursive_searches_nested_dirs() {
        let root = std::env::temp_dir().join(format!("mlbv-test-java-{}", std::process::id()));
        let nested = root.join("jdk-21").join("bin");
        std::fs::create_dir_all(&nested).unwrap();
        let exe = if cfg!(windows) { "javaw.exe" } else { "java" };
        std::fs::write(nested.join(exe), b"fake").unwrap();

        assert_eq!(find_java_exe_recursive(&root, exe), Some(nested.join(exe)));
        assert_eq!(find_java_exe_recursive(&root.join("missing"), exe), None);

        // Best-effort cleanup of the scratch dir; a leftover is harmless.
        let _ = std::fs::remove_dir_all(&root);
    }
}
