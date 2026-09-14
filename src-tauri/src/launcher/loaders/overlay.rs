//! Loader overlay libraries: meta profiles (Fabric/Quilt), installer-produced
//! version JSONs (Forge/NeoForge), and the Modrinth mod fetch they share.
use super::*;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;

/// Fetch a Fabric/Quilt loader profile (a version-JSON overlay).
pub(super) async fn fetch_meta_profile(
    ctx: &Ctx<'_>,
    api_base: &str,
    mc_ver: &str,
    loader_ver: &str,
    label: &str,
) -> Result<FabricProfile> {
    ctx.progress("download", PCT_LOADER, &format!("Fetching {label} Loader profile…"));
    let url = format!("{api_base}/versions/loader/{mc_ver}/{loader_ver}/profile/json");
    let text = ctx.client.get(&url).send().await
        .with_context(|| format!("Cannot reach {label} loader API"))?
        .text().await?;
    serde_json::from_str(&text)
        .map_err(|e| anyhow!("{label} profile parse: {e}"))
}

/// Download the Maven libraries of a Fabric/Quilt profile into the shared cache.
pub(super) async fn download_profile_libraries(
    ctx: &Ctx<'_>,
    profile: &FabricProfile,
    default_base: Option<&str>,
    label: &str,
    concurrent: u32,
) -> Result<Vec<String>> {
    let libs_dir = ctx.shared.join("libraries");
    let mut jobs: Vec<LibDlJob> = Vec::new();
    let mut ordered: Vec<PathBuf> = Vec::new();

    for flib in &profile.libraries {
        // Maven coordinates: group:artifact:version
        let parts: Vec<&str> = flib.name.splitn(3, ':').collect();
        if parts.len() < 3 { continue; }
        let (group, artifact, version) = (parts[0], parts[1], parts[2]);
        let rel_path = format!("{group}/{artifact}/{version}/{artifact}-{version}.jar",
            group = group.replace('.', "/"));
        let dest = libs_dir.join(&rel_path);

        if !is_valid_file(&dest, 0) {
            // Quilt leaves `url` empty for its own artifacts.
            let base = if flib.url.is_empty() {
                default_base.ok_or_else(|| anyhow!("{label} library {} has no download URL", flib.name))?
            } else {
                flib.url.as_str()
            };
            let url = format!("{}/{rel_path}", base.trim_end_matches('/'));
            jobs.push(LibDlJob {
                url,
                path: dest.clone(),
                size: 0,
                sha1: None,
                label: format!("{label} library {}", flib.name),
            });
        }
        ordered.push(dest);
    }

    let stage = format!("{label} libraries");
    download_libs_parallel(ctx, jobs, &stage,
        PCT_LOADER, PCT_LOADER_END - PCT_LOADER, concurrent).await?;

    Ok(ordered.iter().map(|p| p.to_string_lossy().into_owned()).collect())
}

/// One Forge/NeoForge overlay library, classified by where its file comes
/// from. A single classification point for the cache check and the download
/// step, so the two can never disagree about what is required.
pub(super) enum OverlayLib {
    /// Disallowed on this platform, or an entry without usable coordinates:
    /// skip silently (the old behaviour for both cases).
    Skip,
    /// Explicit `downloads.artifact`: download it when absent.
    Explicit { path: PathBuf, url: String, size: u64, sha1: Option<String> },
    /// Plain Maven coordinates the installer must have placed in the cache.
    Cached { path: PathBuf },
}

pub(super) fn classify_overlay_lib(lib_val: &serde_json::Value, libs_dir: &PathBuf) -> OverlayLib {
    let name = lib_val["name"].as_str().unwrap_or("");
    if name.is_empty() { return OverlayLib::Skip; }
    // Overlay rules use the same Mojang semantics as vanilla libraries.
    if let Some(rules_val) = lib_val.get("rules") {
        if let Ok(rules) = serde_json::from_value::<Vec<ArgRule>>(rules_val.clone()) {
            if !rules_allow(&rules) { return OverlayLib::Skip; }
        }
    }
    // Explicit download entry wins when the JSON has one.
    if let (Some(path), Some(url)) = (
        lib_val["downloads"]["artifact"]["path"].as_str(),
        lib_val["downloads"]["artifact"]["url"].as_str(),
    ) {
        if !path.is_empty() && !url.is_empty() {
            return OverlayLib::Explicit {
                path: libs_dir.join(path),
                url: url.to_string(),
                size: lib_val["downloads"]["artifact"]["size"].as_u64().unwrap_or(0),
                sha1: lib_val["downloads"]["artifact"]["sha1"].as_str().map(|s| s.to_string()),
            };
        }
    }
    // Otherwise the installer put it in the cache: resolve Maven coords.
    let parts: Vec<&str> = name.splitn(3, ':').collect();
    if parts.len() >= 3 {
        let jar_name = format!("{}-{}.jar", parts[1], parts[2]);
        let p = libs_dir.join(parts[0].replace('.', "/"))
            .join(parts[1]).join(parts[2]).join(&jar_name);
        OverlayLib::Cached { path: p }
    } else {
        OverlayLib::Skip
    }
}

/// True when every overlay library the launch needs is already in the cache.
/// A cached Forge/NeoForge install that fails this check is re-installed
/// instead of trusted: launching with a partial classpath crashes the game
/// with a confusing error.
pub(super) fn overlay_cache_complete(json: &serde_json::Value, libs_dir: &PathBuf) -> bool {
    let Some(libs) = json["libraries"].as_array() else { return true };
    libs.iter().all(|lib| match classify_overlay_lib(lib, libs_dir) {
        OverlayLib::Skip => true,
        OverlayLib::Explicit { path, .. } | OverlayLib::Cached { path } => path.exists(),
    })
}

/// Collect (and where needed download) the libraries of a Forge/NeoForge
/// overlay JSON. The installer has usually placed them already; anything
/// still missing afterwards is a loud error, never a silent hole in the
/// classpath.
pub(super) async fn download_overlay_libraries(
    ctx: &Ctx<'_>,
    json: &serde_json::Value,
    libs_dir: &PathBuf,
    kind: &str,
    ver_name: &str,
    concurrent: u32,
) -> Result<Vec<String>> {
    let Some(libs) = json["libraries"].as_array() else { return Ok(Vec::new()) };
    let mut jobs: Vec<LibDlJob> = Vec::new();
    let mut ordered: Vec<PathBuf> = Vec::new();

    for lib_val in libs {
        match classify_overlay_lib(lib_val, libs_dir) {
            OverlayLib::Skip => {}
            OverlayLib::Explicit { path, url, size, sha1 } => {
                if !is_valid_file(&path, size) {
                    jobs.push(LibDlJob {
                        url,
                        path: path.clone(),
                        size,
                        sha1,
                        label: format!("{kind} library {}",
                            lib_val["name"].as_str().unwrap_or("?")),
                    });
                }
                ordered.push(path);
            }
            OverlayLib::Cached { path } => {
                ordered.push(path);
            }
        }
    }

    let stage = format!("{kind} libraries");
    download_libs_parallel(ctx, jobs, &stage,
        PCT_LOADER, PCT_LOADER_END - PCT_LOADER, concurrent).await?;

    // Backstop: after a (re-)install every required file must be present.
    // Reaching this means the installer itself failed to provide a library.
    let missing: Vec<String> = ordered.iter()
        .filter(|p| !p.exists())
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    if !missing.is_empty() {
        let shown = missing.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
        let more = if missing.len() > 5 { format!(" (+{} more)", missing.len() - 5) } else { String::new() };
        return Err(anyhow!(
            "{kind} install is incomplete, {n} libraries missing ({shown}{more}). \
             Delete shared/versions/{ver_name} and launch again to re-run the installer.",
            n = missing.len(),
        ));
    }

    Ok(ordered.iter().map(|p| p.to_string_lossy().into_owned()).collect())
}

/// Try to download one Modrinth mod by slug. Silently skips when the project
/// has no build for this Minecraft version / loader — these are optional mods.
pub(super) async fn download_modrinth_mod(
    client: &reqwest::Client,
    slug: &str,
    mc_ver: &str,
    mods_dir: &PathBuf,
    loader: &str,
    bytes_dl: Option<&AtomicU64>,
) {
    let url = format!(
        "https://api.modrinth.com/v2/project/{slug}/version?game_versions=[\"{mc_ver}\"]&loaders=[\"{loader}\"]"
    );
    let resp = match client
        .get(&url)
        .header("User-Agent", MODRINTH_UA)
        .send().await
    {
        Ok(r) => r,
        Err(_) => return,
    };
    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return,
    };
    let first = match json.as_array().and_then(|a| a.first()) {
        Some(v) => v.clone(),
        None => return, // not available for this MC version
    };
    let files = match first["files"].as_array() {
        Some(f) => f.clone(),
        None => return,
    };
    let jar = files.iter().find(|f| {
        f["filename"].as_str().map(|n| n.ends_with(".jar") && !n.contains("-sources") && !n.contains("-javadoc")).unwrap_or(false)
    });
    if let Some(f) = jar {
        if let (Some(dl_url), Some(name)) = (f["url"].as_str(), f["filename"].as_str()) {
            let dest = mods_dir.join(name);
            if !is_valid_file(&dest, 0) {
                // Best-effort: the game starts fine without them, so a failure
                // here is not fatal.
                let _ = download_file(client, dl_url, &dest, 0, None, bytes_dl).await;
            }
        }
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_lib_classification_splits_explicit_cached_and_skipped() {
        let libs = PathBuf::from("/cache/libraries");
        let explicit = serde_json::json!({
            "name": "net.minecraftforge:forge:1.20.1-47.3.11:client",
            "downloads": { "artifact": {
                "path": "net/minecraftforge/forge/1.20.1-47.3.11/forge-1.20.1-47.3.11-client.jar",
                "url": "https://maven.minecraftforge.net/x.jar",
                "size": 123, "sha1": "abc",
            }},
        });
        match classify_overlay_lib(&explicit, &libs) {
            OverlayLib::Explicit { path, url, size, sha1 } => {
                assert_eq!(path, libs.join("net/minecraftforge/forge/1.20.1-47.3.11/forge-1.20.1-47.3.11-client.jar"));
                assert_eq!(url, "https://maven.minecraftforge.net/x.jar");
                assert_eq!(size, 123);
                assert_eq!(sha1.as_deref(), Some("abc"));
            }
            _ => panic!("explicit entry must classify as Explicit"),
        }
        let cached = serde_json::json!({ "name": "org.ow2.asm:asm:9.6" });
        match classify_overlay_lib(&cached, &libs) {
            OverlayLib::Cached { path } => {
                assert_eq!(path, libs.join("org/ow2/asm/asm/9.6/asm-9.6.jar"));
            }
            _ => panic!("coords entry must classify as Cached"),
        }
        // Disallowed on this platform → skipped.
        let denied = serde_json::json!({
            "name": "org.ow2.asm:asm:9.6",
            "rules": [{ "action": "disallow" }],
        });
        assert!(matches!(classify_overlay_lib(&denied, &libs), OverlayLib::Skip));
        // Allowed explicitly → kept.
        let allowed = serde_json::json!({
            "name": "org.ow2.asm:asm:9.6",
            "rules": [{ "action": "allow" }],
        });
        assert!(matches!(classify_overlay_lib(&allowed, &libs), OverlayLib::Cached { .. }));
        // No usable coordinates → skipped (the old silent-skip case).
        assert!(matches!(
            classify_overlay_lib(&serde_json::json!({ "name": "" }), &libs),
            OverlayLib::Skip
        ));
        assert!(matches!(
            classify_overlay_lib(&serde_json::json!({ "name": "not-coords" }), &libs),
            OverlayLib::Skip
        ));
    }
    #[test]
    fn overlay_cache_complete_detects_missing_files() {
        let root = std::env::temp_dir().join(format!("mlbv-test-overlay-{}", std::process::id()));
        let libs = root.join("libraries");
        let json = serde_json::json!({
            "libraries": [
                { "name": "org.ow2.asm:asm:9.6" },
                // Disallowed entries never count as missing.
                { "name": "org.ow2.asm:asm:9.6", "rules": [{ "action": "disallow" }] },
            ],
        });
        assert!(!overlay_cache_complete(&json, &libs));
        let p = libs.join("org/ow2/asm/asm/9.6/asm-9.6.jar");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"x").unwrap();
        assert!(overlay_cache_complete(&json, &libs));
        let _ = std::fs::remove_dir_all(&root);
    }
}
