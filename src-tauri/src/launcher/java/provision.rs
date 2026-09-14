//! Adoptium (Temurin) download + unpack into `shared/java/<major>`.
use super::*;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;
use tauri::Emitter;

pub(super) fn extract_zip_all(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        // Same ZipSlip guard as extract_natives: stay inside dest or skip.
        let Some(out) = zip_entry_path(dest, &name) else { continue; };
        if name.ends_with('/') || name.ends_with('\\') {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() { fs::create_dir_all(p)?; }
        let mut f = fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
    }
    Ok(())
}

/// Download a specific Java major version, emitting `java-progress` events.
/// Used by the setup wizard to download Java 8, 17, and 21 in the background.
pub async fn download_java_major(app: &tauri::AppHandle, major: u32) -> Result<()> {
    let shared_dir = shared_data_dir();
    let exe = if cfg!(windows) { "javaw.exe" } else { "java" };

    let emit = |status: &str, progress: f32, message: &str| {
        let _ = app.emit("java-progress", serde_json::json!({
            "major": major,
            "status": status,
            "progress": progress,
            "message": message,
        }));
    };

    // Only check MLBV's own managed Java — system Java (even Java 25) must not
    // satisfy this check, because v >= r in find_java would skip all downloads.
    let java_dir = shared_dir.join("java").join(format!("jre-{major}"));
    if java_dir.exists() && find_java_exe_recursive(&java_dir, exe).is_some() {
        emit("already", 100.0, "");
        return Ok(());
    }

    emit("downloading", 0.0, "Connecting…");

    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()
        .context("reqwest build")?;

    let os_str   = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch_str = match std::env::consts::ARCH { "x86_64" => "x64", "aarch64" => "aarch64", _ => "x86" };
    // /assets/latest/ is broken (404 for all versions); use /assets/feature_releases/ instead
    let assets_url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{major}/ga?architecture={arch_str}&heap_size=normal&image_type=jre&os={os_str}&vendor=eclipse&page_size=1"
    );

    let assets_json: serde_json::Value = client
        .get(&assets_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot reach Adoptium API")?
        .json().await
        .context("Adoptium assets JSON parse")?;

    let direct_url = assets_json[0]["binaries"][0]["package"]["link"]
        .as_str()
        .ok_or_else(|| anyhow!("Java {major} not found in Adoptium catalog (os={os_str} arch={arch_str})"))?
        .to_string();

    let resp = client.get(&direct_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot download Java from CDN")?;
    if !resp.status().is_success() {
        return Err(anyhow!("Java {major} download HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    // Stream to disk instead of buffering the whole archive in RAM.
    fs::create_dir_all(&java_dir)?;
    let zip_path = java_dir.join("jre.zip");
    {
        use std::io::Write;
        let mut file = std::io::BufWriter::new(fs::File::create(&zip_path)?);
        let mut downloaded: u64 = 0;
        let mut last_mb: u64 = 0;
        let mut resp = resp;
        while let Some(chunk) = resp.chunk().await.context("Download interrupted")? {
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            let mb = downloaded / 1_048_576;
            if mb > last_mb {
                last_mb = mb;
                let pct = if total > 0 {
                    (downloaded as f32 / total as f32) * 90.0
                } else {
                    (mb as f32 * 1.5_f32).min(85.0)
                };
                let tot_s = if total > 0 { format!("/{}", total / 1_048_576) } else { String::new() };
                emit("downloading", pct, &format!("Java {major}: {mb}{tot_s} MB"));
            }
        }
        file.flush()?;
    }

    emit("installing", 92.0, &format!("Installing Java {major}…"));
    extract_zip_all(&zip_path, &java_dir)?;
    let _ = fs::remove_file(&zip_path);

    if find_java_exe_recursive(&java_dir, exe).is_some() {
        emit("done", 100.0, "");
        Ok(())
    } else {
        Err(anyhow!("Java {major} installed but executable not found in extracted archive"))
    }
}
