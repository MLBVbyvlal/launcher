//! Verified streaming downloads, ZIP extraction, path helpers, Mojang rule
//! evaluation.
use super::*;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn mc_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".minecraft")
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(if cfg!(target_os = "macos") {
                "Library/Application Support/minecraft"
            } else {
                ".minecraft"
            })
    }
}

/// Size check for already-cached files. Fresh downloads are verified with
/// SHA-1 in `download_file`; cached files are trusted by size (re-downloading
/// every cached file to re-hash it on each launch would cost bandwidth).
pub(super) fn is_valid_file(path: &Path, expected_size: u64) -> bool {
    if expected_size == 0 { return path.exists(); }
    match fs::metadata(path) {
        Ok(m) => m.len() == expected_size,
        Err(_) => false,
    }
}

/// Stream a URL to `path`, verifying size and (when known) SHA-1, and count
/// the bytes into the shared speed counter.
///
/// Writes to a `.part` file first and renames on success, so an interrupted
/// download can never leave a corrupt file at the real path — `is_valid_file`
/// would trust a half-written file forever.
pub(super) async fn download_file(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    expected_size: u64,
    expected_sha1: Option<&str>,
    bytes_dl: Option<&AtomicU64>,
) -> Result<()> {
    use sha1::Digest;
    use std::io::Write;

    let resp = client.get(url).send().await?
        .error_for_status()
        .with_context(|| format!("HTTP error downloading {url}"))?;

    if let Some(p) = path.parent() { fs::create_dir_all(p)?; }
    let part_path = path.with_extension("part");
    // Buffered: HTTP chunks are small and one syscall per chunk is slow.
    let mut file = std::io::BufWriter::new(fs::File::create(&part_path)?);
    let mut hasher = sha1::Sha1::new();
    let mut downloaded: u64 = 0;
    let mut resp = resp;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk)?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if let Some(counter) = bytes_dl {
            counter.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
    }
    // Explicit flush: BufWriter's drop ignores write errors, and a short
    // file must fail the size check below, not pass it.
    file.flush()?;
    drop(file);

    if expected_size > 0 && downloaded != expected_size {
        let _ = fs::remove_file(&part_path);
        return Err(anyhow!(
            "Size mismatch for {url}: got {downloaded}, expected {expected_size}"
        ));
    }
    if let Some(expected) = expected_sha1 {
        let digest = format!("{:x}", hasher.finalize());
        if !digest.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&part_path);
            return Err(anyhow!(
                "SHA-1 mismatch for {url}: got {digest}, expected {expected}"
            ));
        }
    }
    fs::rename(&part_path, path)
        .with_context(|| format!("Finalizing {}", path.display()))?;
    Ok(())
}

/// Pre-1.7.3 asset indexes (`map_to_resources: true`) require every asset to
/// also be reachable at `assets/virtual/legacy/<index key>` — without this the
/// old versions start without textures or sounds. Hard links (same volume,
/// zero extra space), falling back to a copy.
pub(super) fn map_legacy_assets(objects: &HashMap<String, AssetObj>, objs_dir: &Path) -> Result<()> {
    let Some(assets_root) = objs_dir.parent() else { return Ok(()) };
    let legacy_dir = assets_root.join("virtual").join("legacy");
    for (path, obj) in objects {
        let src = objs_dir.join(&obj.hash[..2]).join(&obj.hash);
        if !src.exists() { continue; }
        let dst = legacy_dir.join(path);
        if dst.exists() { continue; }
        if let Some(p) = dst.parent() { fs::create_dir_all(p)?; }
        fs::hard_link(&src, &dst).or_else(|_| fs::copy(&src, &dst).map(|_| ()))?;
    }
    Ok(())
}

/// Join a ZIP entry name onto `dest`, rejecting anything that would escape
/// it (ZipSlip: `../`, absolute paths, Windows prefixes). A lexical
/// `starts_with` check is not enough — `dest.join("../x")` still starts with
/// `dest` as a string — so every component must be a plain segment.
pub(super) fn zip_entry_path(dest: &Path, name: &str) -> Option<PathBuf> {
    let mut out = dest.to_path_buf();
    for comp in Path::new(name).components() {
        match comp {
            Component::Normal(seg) => out.push(seg),
            _ => return None,
        }
    }
    Some(out)
}

pub(super) fn extract_natives(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.starts_with("META-INF") || name.ends_with('/') { continue; }
        // Skip entries that would escape the natives dir (ZipSlip) instead of
        // failing the whole launch — one hostile entry must not break the rest.
        let Some(out) = zip_entry_path(dest, &name) else { continue; };
        if let Some(p) = out.parent() { fs::create_dir_all(p)?; }
        let mut f = fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
    }
    Ok(())
}

pub(super) fn os_condition_matches(os: &OsCondition) -> bool {
    let name_ok = match &os.name {
        None => true,
        Some(n) => (cfg!(windows) && n == "windows") ||
                   (cfg!(target_os = "macos") && n == "osx") ||
                   (cfg!(target_os = "linux") && n == "linux"),
    };
    let arch_ok = match &os.arch {
        None => true,
        Some(a) => match a.as_str() {
            // "x86" means 32-bit only — exclude on 64-bit JVM (avoids UnsatisfiedLinkError)
            "x86" => cfg!(target_pointer_width = "32"),
            "x86_64" | "amd64" => cfg!(target_pointer_width = "64"),
            _ => true,
        },
    };
    name_ok && arch_ok
}

/// Mojang rule semantics shared by vanilla libraries, launch arguments and
/// Forge/NeoForge overlay libraries: no rules means allowed, otherwise the
/// last matching rule wins.
pub(super) fn rules_allow(rules: &[ArgRule]) -> bool {
    if rules.is_empty() { return true; }
    let mut allowed = false;
    for rule in rules {
        let os_match = match &rule.os {
            None => true,
            Some(os) => os_condition_matches(os),
        };
        if os_match { allowed = rule.action == "allow"; }
    }
    allowed
}

pub(super) fn lib_allowed(lib: &Library) -> bool {
    rules_allow(&lib.rules)
}

pub(super) fn resolve_arg(arg: &Arg, replace: &impl Fn(&str) -> String, out: &mut Vec<String>) {
    match arg {
        Arg::Plain(s) => out.push(replace(s)),
        Arg::Conditional { rules, value } => {
            // Skip args that require specific features (demo, custom resolution)
            if rules.iter().any(|r| r.features.is_some()) { return; }
            if !rules_allow(rules) { return; }

            match value {
                ArgValue::One(s) => out.push(replace(s)),
                ArgValue::Many(v) => out.extend(v.iter().map(|s| replace(s))),
            }
        }
    }
}

pub(super) fn os_classifier_key() -> &'static str {
    if cfg!(windows) { "windows" }
    else if cfg!(target_os = "macos") { "osx" }
    else { "linux" }
}

pub(super) fn arch_bits() -> &'static str {
    if cfg!(target_pointer_width = "64") { "64" } else { "32" }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_entry_path_blocks_escapes() {
        let dest = std::env::temp_dir().join("mlbv-test-zip");
        assert_eq!(
            zip_entry_path(&dest, "linux/x86_64/lib.so"),
            Some(dest.join("linux").join("x86_64").join("lib.so"))
        );
        // Only portable cases: backslash and drive-letter handling differs
        // between Windows and Unix, but these three are rejected everywhere.
        for evil in ["../evil.dll", "a/../../evil.dll", "/abs/evil.dll"] {
            assert_eq!(zip_entry_path(&dest, evil), None, "{evil:?} must not escape");
        }
    }
    #[test]
    fn rules_allow_matches_mojang_semantics() {
        let allow = ArgRule { action: "allow".to_string(), os: None, features: None };
        let deny = ArgRule { action: "disallow".to_string(), os: None, features: None };
        let alien = ArgRule {
            action: "allow".to_string(),
            os: Some(OsCondition { name: Some("solaris-never".to_string()), arch: None }),
            features: None,
        };
        assert!(rules_allow(&[]));
        assert!(rules_allow(std::slice::from_ref(&allow)));
        assert!(!rules_allow(std::slice::from_ref(&deny)));
        // Last matching rule wins.
        assert!(rules_allow(&[deny.clone(), allow.clone()]));
        assert!(!rules_allow(&[allow.clone(), deny.clone()]));
        // A rule for another OS never matches, on any platform.
        assert!(!rules_allow(std::slice::from_ref(&alien)));
        // ...and does not shadow a matching rule either way.
        assert!(rules_allow(&[alien.clone(), allow.clone()]));
    }
}
