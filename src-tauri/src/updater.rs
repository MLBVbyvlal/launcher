//! Self-update: GitHub release lookup, verified installer download, apply.
//! Windows-only by design (NSIS/MSI).
use tauri::Emitter;

// ── Update check ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub(crate) struct ReleaseInfo {
    version:  String,
    tag_name: String,
    body:     String,
    html_url: String,
    asset_url: String,
    msi_url:  String,
    /// `sha256:<hex>` digests GitHub publishes per asset; empty when absent.
    asset_sha256: String,
    msi_sha256:   String,
    unstable_warning: bool,
}

fn strip_type_prefix(v: &str) -> &str {
    let v = v.trim_start_matches('v');
    for p in &["beta", "pre-release", "release"] {
        if let Some(rest) = v.strip_prefix(p) { return rest; }
    }
    v
}

fn version_type(v: &str) -> &'static str {
    let v = v.trim_start_matches('v');
    if v.starts_with("beta") { "beta" }
    else if v.starts_with("pre-release") { "pre-release" }
    else { "release" }
}

fn parse_semver(v: &str) -> (u64, u64, u64) {
    let v = strip_type_prefix(v);
    let mut parts = v.splitn(3, '.').map(|p| p.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

#[tauri::command]
pub(crate) async fn check_for_update() -> Result<Option<ReleaseInfo>, String> {
    // The updater ships an NSIS .exe (silent) or a WiX .msi (setup wizard) —
    // Windows-only by design. The frontend skips the check elsewhere; this is
    // the backstop.
    if !cfg!(windows) {
        return Err("Self-update is only supported on Windows.".to_string());
    }
    let current = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;
    let releases: Vec<serde_json::Value> = client
        .get("https://api.github.com/repos/MLBVbyvlal/launcher/releases")
        .header("Accept", "application/vnd.github+json")
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    // Consider every non-draft release (all published releases of this
    // project are marked pre-release, so filtering them out — the old
    // behaviour — made the updater return "No releases found" forever).
    // Pick the highest version that is actually newer than the running one;
    // pre-release candidates are surfaced with a warning instead of hidden.
    let mut candidates: Vec<&serde_json::Value> = releases.iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .collect();
    candidates.sort_by(|a, b| {
        let va = parse_semver(a["tag_name"].as_str().unwrap_or(""));
        let vb = parse_semver(b["tag_name"].as_str().unwrap_or(""));
        vb.cmp(&va)
    });
    // Being up to date is not an error: both frontend call sites already
    // treat `null` as "up to date", so return None instead of failing.
    let Some(release) = candidates.iter()
        .find(|r| parse_semver(r["tag_name"].as_str().unwrap_or("")) > parse_semver(current))
    else {
        return Ok(None);
    };

    let tag = release["tag_name"].as_str().unwrap_or("").to_string();
    let ver = tag.trim_start_matches('v');
    let find_asset = |ext: &str| -> (String, String) {
        release["assets"]
            .as_array()
            .and_then(|a| {
                a.iter().find(|asset| {
                    asset["name"]
                        .as_str()
                        .map(|n| n.ends_with(ext))
                        .unwrap_or(false)
                })
            })
            .map(|a| (
                a["browser_download_url"].as_str().unwrap_or("").to_string(),
                digest_hex(a["digest"].as_str().unwrap_or("")),
            ))
            .unwrap_or_default()
    };
    // NSIS .exe = silent one-click update (recommended); WiX .msi = guided
    // install through the Windows Installer service. Either may be absent on
    // old hand-assembled releases — the frontend only offers what exists.
    let (asset_url, asset_sha256) = find_asset(".exe");
    let (msi_url, msi_sha256) = find_asset(".msi");

    let unstable_warning = release["prerelease"].as_bool().unwrap_or(false) || version_type(ver) != "release";

    Ok(Some(ReleaseInfo {
        version:  ver.to_string(),
        tag_name: tag,
        body:     release["body"].as_str().unwrap_or("").to_string(),
        html_url: release["html_url"].as_str().unwrap_or("").to_string(),
        asset_url,
        msi_url,
        asset_sha256,
        msi_sha256,
        unstable_warning,
    }))
}

/// GitHub's asset digest is `sha256:<64 hex>`; anything else is treated as
/// "no digest" so a malformed field cannot pass as a match.
fn digest_hex(digest: &str) -> String {
    let hex = digest.trim().strip_prefix("sha256:").unwrap_or("").to_ascii_lowercase();
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) { hex } else { String::new() }
}

// Download the update installer to %TEMP% and emit progress events
/// Only GitHub release hosts are trusted for update downloads. The URL comes
/// from our own release metadata, but `download_update` fetches an executable
/// and `apply_update` runs it — so an arbitrary URL must never be accepted,
/// even if the release JSON was tampered with.
fn update_host_allowed(url: &str) -> bool {
    const TRUSTED_HOSTS: [&str; 3] = [
        "github.com",
        "objects.githubusercontent.com",
        "release-assets.githubusercontent.com",
    ];
    let Ok(parsed) = url::Url::parse(url) else { return false };
    if parsed.scheme() != "https" { return false; }
    let Some(host) = parsed.host_str() else { return false; };
    let host = host.to_ascii_lowercase();
    TRUSTED_HOSTS.contains(&host.as_str())
}

/// Installer kind of an update URL — `exe` (NSIS) or `msi` (WiX) — taken
/// from the file name at the end of the URL. Anything else is rejected, so
/// `download_update` and `apply_update` can never disagree about the file.
fn installer_ext_from_url(url: &str) -> Option<&str> {
    // Signed asset URLs carry a query string (?token=…); strip it first —
    // the file name is the last segment of the path, not of the whole URL.
    let path = url.split('?').next().unwrap_or("");
    let file_name = path.rsplit('/').next().unwrap_or("");
    match file_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "exe" => Some("exe"),
        "msi" => Some("msi"),
        _ => None,
    }
}

/// `sha256` is the expected digest of the installer (hex). It is required:
/// the file is executed afterwards, so an asset without a digest is refused
/// rather than run unverified.
#[tauri::command]
pub(crate) async fn download_update(app: tauri::AppHandle, url: String, sha256: String) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Self-update is only supported on Windows.".to_string());
    }
    if !update_host_allowed(&url) {
        return Err(format!("Refusing to download update from untrusted host: {url}"));
    }
    let expected = digest_hex(&format!("sha256:{sha256}"));
    if expected.is_empty() {
        return Err("Release asset has no SHA-256 digest — refusing unverified update.".to_string());
    }
    let Some(ext) = installer_ext_from_url(&url) else {
        return Err(format!("Unsupported installer type in update URL: {url}"));
    };
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let final_path = std::env::temp_dir().join(format!("mlbv-update.{ext}"));
    // Stream to a .part file and only rename once the digest matched, so a
    // half-written or tampered file never sits under the name apply_update runs.
    let part_path = std::env::temp_dir().join(format!("mlbv-update.{ext}.part"));
    let mut file = std::fs::File::create(&part_path)
        .map_err(|e| format!("Cannot create temp file: {e}"))?;
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    let mut downloaded: u64 = 0;
    let mut resp = resp;

    while let Some(chunk) = resp.chunk().await
        .map_err(|e| format!("Download interrupted: {e}"))?
    {
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| format!("Write failed: {e}"))?;
        sha2::Digest::update(&mut hasher, &chunk);
        downloaded += chunk.len() as u64;
        if total > 0 {
            let pct = downloaded as f32 / total as f32 * 100.0;
            let _ = app.emit("update-progress", serde_json::json!({ "percent": pct }));
        }
    }
    drop(file);
    let actual = format!("{:x}", sha2::Digest::finalize(hasher));
    if actual != expected {
        let _ = std::fs::remove_file(&part_path);
        return Err("Downloaded installer failed SHA-256 verification — update aborted.".to_string());
    }
    std::fs::rename(&part_path, &final_path).map_err(|e| format!("Cannot finalize update file: {e}"))?;
    Ok(())
}

// Run the downloaded installer, then exit. `kind` is "exe" (silent NSIS) or
// "msi" (interactive wizard through the Windows Installer service).
#[tauri::command]
pub(crate) fn apply_update(app: tauri::AppHandle, new_version: String, kind: String) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Self-update is only supported on Windows.".to_string());
    }

    // Write the new version so the next launch can show "Updated to vX" toast
    let marker = std::env::temp_dir().join("mlbv-just-updated.txt");
    let _ = std::fs::write(&marker, &new_version);

    match kind.as_str() {
        "msi" => {
            let msi_path = std::env::temp_dir().join("mlbv-update.msi");
            if !msi_path.exists() {
                return Err("Update package not found".to_string());
            }
            std::process::Command::new("msiexec")
                .arg("/i")
                .arg(&msi_path)
                .spawn()
                .map_err(|e| format!("Failed to start installer: {e}"))?;
        }
        "exe" => {
            let tmp_path = std::env::temp_dir().join("mlbv-update.exe");
            if !tmp_path.exists() {
                return Err("Update installer not found".to_string());
            }

            let install_dir = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .parent()
                .ok_or_else(|| "Cannot determine install directory".to_string())?
                .to_path_buf();

            // NSIS silent install: /S = silent, /D= = destination (must be last, no quotes)
            std::process::Command::new(&tmp_path)
                .arg("/S")
                .arg(format!("/D={}", install_dir.to_string_lossy()))
                .spawn()
                .map_err(|e| format!("Failed to start installer: {e}"))?;
        }
        other => return Err(format!("Unknown installer kind: {other}")),
    }

    std::thread::sleep(std::time::Duration::from_millis(400));
    app.exit(0);
    Ok(())
}

// Check if we just updated — returns old version string, or empty if not
#[tauri::command]
pub(crate) fn get_just_updated() -> String {
    let marker = std::env::temp_dir().join("mlbv-just-updated.txt");
    if !marker.exists() { return String::new(); }
    let ver = std::fs::read_to_string(&marker).unwrap_or_default();
    let _ = std::fs::remove_file(&marker);
    ver.trim().to_string()
}

#[tauri::command]
pub(crate) fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Cancel the launch of one instance; other launches in flight are untouched.


// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_type_prefix_handles_all_tag_shapes() {
        assert_eq!(strip_type_prefix("v1.2.3"), "1.2.3");
        assert_eq!(strip_type_prefix("1.2.3"), "1.2.3");
        assert_eq!(strip_type_prefix("beta0.0.4"), "0.0.4");
        assert_eq!(strip_type_prefix("vbeta0.0.4"), "0.0.4");
        assert_eq!(strip_type_prefix("pre-release1.0.0"), "1.0.0");
        assert_eq!(strip_type_prefix("release2.1.3"), "2.1.3");
    }
    #[test]
    fn version_type_classifies_tags() {
        assert_eq!(version_type("vbeta0.0.4"), "beta");
        assert_eq!(version_type("beta1.0.0"), "beta");
        assert_eq!(version_type("pre-release1.0.0"), "pre-release");
        assert_eq!(version_type("vpre-release1.0.0"), "pre-release");
        assert_eq!(version_type("v1.2.3"), "release");
        assert_eq!(version_type("release1.0.0"), "release");
    }
    #[test]
    fn parse_semver_orders_releases() {
        assert_eq!(parse_semver("v1.2.3"), (1, 2, 3));
        assert_eq!(parse_semver("beta0.0.4"), (0, 0, 4));
        assert_eq!(parse_semver("21.1.172"), (21, 1, 172));
        assert_eq!(parse_semver("1.20"), (1, 20, 0));
        assert_eq!(parse_semver("garbage"), (0, 0, 0));
        assert!(parse_semver("beta0.0.4") < parse_semver("0.0.5"));
        assert_eq!(parse_semver("v0.0.4"), parse_semver("beta0.0.4"));
    }
    #[test]
    fn digest_hex_accepts_only_sha256_prefixed_hex() {
        let h = "a".repeat(64);
        assert_eq!(digest_hex(&format!("sha256:{h}")), h);
        assert_eq!(digest_hex(&format!("SHA256:{h}")), "");
        assert_eq!(digest_hex(&format!("sha256:{}", "a".repeat(63))), "");
        assert_eq!(digest_hex(&format!("sha256:{}", "g".repeat(64))), "");
        assert_eq!(digest_hex(""), "");
    }
    #[test]
    fn update_host_allowed_trusts_github_release_hosts_only() {
        assert!(update_host_allowed(
            "https://github.com/MLBVbyvlal/launcher/releases/download/v0.0.4/MLBV_setup.exe"
        ));
        assert!(update_host_allowed(
            "https://objects.githubusercontent.com/abc123/MLBV_setup.exe"
        ));
        assert!(update_host_allowed(
            "https://release-assets.githubusercontent.com/abc123/MLBV_setup.exe"
        ));
        assert!(update_host_allowed(
            "https://GITHUB.COM/MLBVbyvlal/launcher/releases/download/v0.0.4/x.exe"
        ));
        for bad in [
            "https://evil.com/MLBV_setup.exe",
            "https://github.com.evil.com/x.exe",
            "http://github.com/MLBVbyvlal/launcher/releases/download/v0.0.4/x.exe",
            "not a url",
            "",
            "file:///C:/Windows/evil.exe",
        ] {
            assert!(!update_host_allowed(bad), "{bad:?} must be rejected");
        }
    }
    #[test]
    fn installer_ext_from_url_accepts_exe_and_msi_only() {
        assert_eq!(
            installer_ext_from_url(
                "https://github.com/MLBVbyvlal/launcher/releases/download/v0.0.5/MLBV_0.0.5_x64-setup.exe"
            ),
            Some("exe")
        );
        assert_eq!(
            installer_ext_from_url(
                "https://objects.githubusercontent.com/abc/MLBV_0.0.5_x64_en-US.msi?token=zz"
            ),
            Some("msi")
        );
        assert_eq!(
            installer_ext_from_url("https://github.com/x/y/releases/download/v1/setup.EXE"),
            Some("exe")
        );
        for bad in [
            "https://example.com/setup.zip",
            "https://example.com/noext",
            "https://example.com/fake.exe/real.msi.bak",
            "not a url",
            "",
        ] {
            assert_eq!(installer_ext_from_url(bad), None, "{bad:?} must be rejected");
        }
    }
}
