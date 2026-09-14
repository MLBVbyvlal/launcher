//! LiquidBounce specifics: the mods its launch manifest lists, Fabric API,
//! and the JCEF WebView package.
use super::*;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// ─── Loader mods ─────────────────────────────────────────────────────────────

/// Fabric API from Modrinth (project `P7dR8mSH`), version-matched.
pub(super) async fn download_fabric_api(ctx: &Ctx<'_>, mc_ver: &str) -> Result<()> {
    ctx.progress("download", PCT_MODS, "Downloading Fabric API from Modrinth…");
    let url = format!(
        "https://api.modrinth.com/v2/project/P7dR8mSH/version?game_versions=[\"{mc_ver}\"]&loaders=[\"fabric\"]"
    );
    let versions: serde_json::Value = ctx.client
        .get(&url)
        .header("User-Agent", MODRINTH_UA)
        .send().await
        .context("Cannot reach Modrinth API")?
        .json().await?;

    let mods_dir = ctx.game_dir.join("mods");
    let files = versions.as_array()
        .and_then(|a| a.first())
        .and_then(|v| v["files"].as_array());
    let jar = files.and_then(|files| files.iter().find(|f| {
        f["filename"].as_str()
            .map(|n| n.ends_with(".jar") && !n.contains("-sources"))
            .unwrap_or(false)
    }));
    if let Some(f) = jar {
        if let (Some(url), Some(name)) = (f["url"].as_str(), f["filename"].as_str()) {
            let dest = mods_dir.join(name);
            if !is_valid_file(&dest, 0) {
                download_file(ctx.client, url, &dest, 0, None, Some(&ctx.ctl.bytes)).await
                    .with_context(|| format!("Downloading {}", dest.display()))?;
            }
        }
    }
    Ok(())
}

/// LiquidBounce mod set from the launch manifest.
pub(super) async fn download_lb_mods(ctx: &Ctx<'_>, manifest: &LbManifest) -> Result<()> {
    let mods_dir = ctx.game_dir.join("mods");
    let total_mods = manifest.mods.len();

    for (i, m) in manifest.mods.iter().enumerate() {
        if !m.required { continue; }
        ctx.gate().await?;
        let pct = PCT_MODS + (i as f32 / total_mods.max(1) as f32) * 2.0;
        ctx.progress("download", pct, &format!("Downloading: {}…", m.name));

        let src_type = m.source.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match src_type {
            "skip" => {
                // New API: source has "url" + "artifactName".
                // Old API: source has "skip_pid" or "pid".
                let download_url = m.source.get("url")
                    .and_then(|v| v.as_str())
                    .map(|u| u.to_string())
                    .or_else(|| {
                        let pid = m.source.get("skip_pid")
                            .or_else(|| m.source.get("pid"))
                            .and_then(|v| v.as_str())?;
                        Some(format!("https://api.liquidbounce.net/api/v3/file/{}", pid))
                    });

                if let Some(dl_url) = download_url {
                    let dest_name = m.source.get("artifactName")
                        .and_then(|v| v.as_str())
                        .map(|a| format!("{a}.jar"))
                        .unwrap_or_else(|| format!("{}.jar", m.name.replace(' ', "_")));
                    let dest = mods_dir.join(&dest_name);

                    if !is_valid_file(&dest, 0) {
                        ctx.progress("download", pct,
                            &format!("Opening download page for {}… click «Download»", m.name));
                        download_lb_mod_webview(ctx.app, &ctx.instance, &dl_url, &dest).await
                            .with_context(|| format!("Failed to download mod {}", m.name))?;
                    }
                }
            }
            "repository" => {
                let repo     = m.source.get("repository").and_then(|v| v.as_str()).unwrap_or("");
                let artifact = m.source.get("artifact").and_then(|v| v.as_str()).unwrap_or("");
                // Base URL: from the manifest repositories map, or well-known fallbacks
                let base = manifest.repositories.get(repo)
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| match repo {
                        "modrinth" => "https://api.modrinth.com/maven/",
                        "fabric"   => "https://maven.fabricmc.net/",
                        _          => "https://maven.liquidbounce.net/repo/",
                    });
                let parts: Vec<&str> = artifact.splitn(3, ':').collect();
                if parts.len() >= 3 {
                    let (group, art, ver) = (parts[0], parts[1], parts[2]);
                    let jar_name = format!("{art}-{ver}.jar");
                    let dest = mods_dir.join(&jar_name);
                    if !is_valid_file(&dest, 0) {
                        let url = format!("{}/{}/{art}/{ver}/{jar_name}",
                            base.trim_end_matches('/'), group.replace('.', "/"));
                        download_file(ctx.client, &url, &dest, 0, None, Some(&ctx.ctl.bytes)).await
                            .with_context(|| format!("Downloading {}", dest.display()))?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Optional quality-of-life mods shipped with a LiquidBounce instance.
pub(super) async fn download_lb_extra_mods(ctx: &Ctx<'_>, mc_ver: &str) {
    const EXTRA_MODS: [&str; 7] = [
        "sodium", "modmenu", "immediatelyfast", "iris", "lithium",
        "viafabricplus", "exploitfixer",
    ];
    let mods_dir = ctx.game_dir.join("mods");
    for (i, slug) in EXTRA_MODS.iter().enumerate() {
        let pct = 90.0 + (i as f32 / EXTRA_MODS.len() as f32) * 2.0;
        ctx.progress("download", pct, &format!("Extra mods: {slug}…"));
        // Best-effort: these are optional, the instance launches without them.
        download_modrinth_mod(ctx.client, slug, mc_ver, &mods_dir, "fabric", Some(&ctx.ctl.bytes)).await;
    }
}

// ─── LB mod WebView download ──────────────────────────────────────────────────

/// Open the LiquidBounce download-queue page in a popup.
/// Uses Tauri's on_download to intercept the browser-initiated file download
/// (WebView2 fires a download event, NOT a navigation, for Content-Disposition:attachment).
/// Redirects the save path to a temp file, polls size for progress, extracts the JAR.
pub(super) async fn download_lb_mod_webview(
    app: &tauri::AppHandle,
    instance: &str,
    queue_url: &str,
    dest: &Path,
) -> Result<()> {
    // Close any stale window from a previous attempt
    if let Some(old) = app.get_webview_window("lb-dl") {
        let _ = old.close();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    let temp_path   = std::env::temp_dir().join("mlbv_lb_mod.tmp");
    let temp_cb     = temp_path.clone();

    let started = Arc::new(AtomicBool::new(false));
    let done    = Arc::new(AtomicBool::new(false));
    let errored = Arc::new(AtomicBool::new(false));
    let closed  = Arc::new(AtomicBool::new(false));

    let started_cb = started.clone();
    let done_cb    = done.clone();
    let err_cb     = errored.clone();
    let closed_ev  = closed.clone();

    let parsed = url::Url::parse(queue_url)
        .map_err(|e| anyhow!("Invalid URL: {e}"))?;

    let win = tauri::WebviewWindowBuilder::new(app, "lb-dl", tauri::WebviewUrl::External(parsed))
        .title("LiquidBounce — click «Download»")
        .inner_size(960.0, 680.0)
        .center()
        .always_on_top(true)
        .on_download(move |_webview, event| {
            use tauri::webview::DownloadEvent;
            match event {
                DownloadEvent::Requested { destination, .. } => {
                    // Redirect the download to our temp file instead of the browser default
                    *destination = temp_cb.clone();
                    started_cb.store(true, Ordering::Relaxed);
                    true // allow the download
                }
                DownloadEvent::Finished { success, .. } => {
                    if success {
                        done_cb.store(true, Ordering::Relaxed);
                    } else {
                        err_cb.store(true, Ordering::Relaxed);
                    }
                    true
                }
                _ => true,
            }
        })
        .build()
        .map_err(|e| anyhow!("Failed to open window: {e}"))?;

    win.on_window_event({
        let c = closed_ev.clone();
        let started_ref = started.clone();
        move |ev| {
            match ev {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if started_ref.load(Ordering::Relaxed) {
                        api.prevent_close();
                    } else {
                        c.store(true, Ordering::Relaxed);
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    c.store(true, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    });

    let start = std::time::Instant::now();
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if done.load(Ordering::Relaxed) { break; }

        if errored.load(Ordering::Relaxed) {
            let _ = win.close();
            return Err(anyhow!("Failed to download mod in browser"));
        }
        // Window closed before the download even started → user cancelled
        if closed.load(Ordering::Relaxed) && !started.load(Ordering::Relaxed) {
            return Err(anyhow!("Mod download cancelled by user"));
        }

        // Show file-size progress while WebView downloads
        if started.load(Ordering::Relaxed) {
            let sz_mb = fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0) / 1024 / 1024;
            let pct   = (77.0_f32 + sz_mb as f32).min(88.0);
            progress(app, instance, "download", pct, &format!("Downloading LiquidBounce… {sz_mb} MB"));
        }

        if start.elapsed().as_secs() > 300 {
            let _ = win.close();
            return Err(anyhow!("Timed out (5 min)"));
        }
    }
    let _ = win.close();

    // Read the completed download and process it
    let bytes = fs::read(&temp_path).context("Reading downloaded file")?;
    let _ = fs::remove_file(&temp_path); // cleanup regardless

    if bytes.len() < 4 || !bytes.starts_with(b"PK") {
        return Err(anyhow!("Downloaded file is not a ZIP/JAR"));
    }

    // Try to extract a .jar from inside the ZIP; if none found, the file itself is the JAR
    match extract_jar_from_zip(&bytes, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::write(dest, &bytes).context("Saving JAR")?;
            Ok(())
        }
    }
}

/// Extract the first .jar found inside a ZIP archive.
pub(super) fn extract_jar_from_zip(zip_bytes: &[u8], dest: &Path) -> Result<()> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let mut jar_idx: Option<usize> = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name  = entry.name().to_string();
        drop(entry);
        if name.ends_with(".jar") {
            if !name.contains('/') && !name.contains('\\') {
                jar_idx = Some(i);
                break; // prefer root-level
            } else if jar_idx.is_none() {
                jar_idx = Some(i);
            }
        }
    }

    let idx = jar_idx.ok_or_else(|| anyhow!("No .jar file found in ZIP"))?;
    let mut entry = archive.by_index(idx)?;
    let mut data  = Vec::new();
    entry.read_to_end(&mut data)?;
    fs::write(dest, data)?;
    Ok(())
}
