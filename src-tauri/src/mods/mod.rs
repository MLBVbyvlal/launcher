//! Mod discovery and management (Modrinth + CurseForge).
//!
//! PrismLauncher is the reference UX this mirrors: search either platform
//! from inside the launcher, pick a file built for the instance's MC version
//! + loader, install it, and later check every installed mod for updates in
//! one click. Two deliberate divergences:
//!
//! - CurseForge needs the user's own API key (Settings → General). Prism
//!   ships an official project key from Overwolf; we have none, so the key
//!   field + a pointer to console.curseforge.com is the honest substitute.
//! - Update checks compare the installed version id against the newest file
//!   for this MC/loader instead of hashing jars Prism-style — there is no
//!   sha512 crate in the tree and the lockfile cannot grow offline.
//!
//! Installed-through-the-launcher mods are tracked in `mods/.mlbv-mods.json`
//! (`filename → ModMeta`); jars dropped in by hand have no entry and are
//! shown as manually added (no update checks for those). Disabling a mod
//! renames `x.jar` ⇄ `x.jar.disabled`, the same convention Prism uses.
//!
//! Layout: this file holds the shared shapes and the Tauri commands;
//! `platforms.rs` talks to Modrinth/CurseForge; `local.rs` owns the
//! on-disk index and the jar files inside an instance.
use super::launcher::valid_instance_name;

mod local;
mod platforms;

pub(crate) use local::forget_mods;
use local::*;
use platforms::*;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const CURSEFORGE_API: &str = "https://api.curseforge.com/v1";
// Modrinth rejects requests without a descriptive User-Agent.
const UA: &str = concat!("MLBV/", env!("CARGO_PKG_VERSION"), " (github.com/MLBVbyvlal/launcher)");

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(|e| e.to_string())
}
// ── shared shapes ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ModHit {
    pub(super) id: String,
    pub(super) slug: String,
    pub(super) name: String,
    pub(super) author: String,
    pub(super) summary: String,
    pub(super) icon_url: String,
    pub(super) downloads: u64,
    pub(super) updated: String,
}

#[derive(serde::Serialize)]
pub struct ModSearchResult {
    pub(super) items: Vec<ModHit>,
    pub(super) total: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct ModFile {
    pub(super) version_id: String,
    pub(super) name: String,
    pub(super) version_number: String,
    pub(super) mc_versions: Vec<String>,
    pub(super) loaders: Vec<String>,
    /// "release" | "beta" | "alpha".
    pub(super) release_type: String,
    pub(super) file_name: String,
    pub(super) download_url: String,
    pub(super) size: u64,
    pub(super) published: String,
}

/// One row of the per-instance mod index (`mods/.mlbv-mods.json`).
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct ModMeta {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version_id: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub icon_url: String,
}

#[derive(serde::Serialize, Clone)]
pub struct ModEntry {
    pub(super) filename: String,
    pub(super) enabled: bool,
    pub(super) size: u64,
    #[serde(flatten)]
    pub(super) meta: ModMeta,
}

#[derive(serde::Serialize)]
pub struct ModUpdate {
    pub(super) filename: String,
    pub(super) name: String,
    pub(super) current: String,
    pub(super) latest: String,
    pub(super) version_id: String,
    pub(super) file_name: String,
    pub(super) download_url: String,
    pub(super) size: u64,
    pub(super) source: String,
    pub(super) project_id: String,
    pub(super) author: String,
    pub(super) icon_url: String,
}
// ── filters ───────────────────────────────────────────────────────────────────

/// Our loader id → (Modrinth category facet, CurseForge modLoaderType).
fn loader_filter(loader: &str) -> (Option<&'static str>, Option<u32>) {
    match loader {
        "fabric" => (Some("fabric"), Some(4)),
        "quilt" => (Some("quilt"), Some(5)),
        "forge" => (Some("forge"), Some(1)),
        "neoforge" => (Some("neoforge"), Some(6)),
        _ => (None, None),
    }
}

/// Empty / "latest" (unresolved instance) means "don't filter by version".
fn mc_filter(mc: &str) -> Option<&str> {
    let mc = mc.trim();
    if mc.is_empty() || mc == "latest" {
        None
    } else {
        Some(mc)
    }
}
// ── search ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn search_mods(
    source: String,
    query: String,
    mc_version: String,
    loader: String,
    cf_key: String,
    limit: u32,
    offset: u32,
) -> Result<ModSearchResult, String> {
    match source.as_str() {
        "modrinth" => search_modrinth(&query, &mc_version, &loader, limit, offset).await,
        "curseforge" => {
            search_curseforge(&query, &mc_version, &loader, &cf_key, limit, offset).await
        }
        other => Err(format!("Unknown mod source: {other}")),
    }
}

// ── versions ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_mod_versions(
    source: String,
    project_id: String,
    mc_version: String,
    loader: String,
    cf_key: String,
) -> Result<Vec<ModFile>, String> {
    mod_versions(&source, &project_id, &mc_version, &loader, &cf_key).await
}

// ── installed-mod listing ─────────────────────────────────────────────────────

#[tauri::command]
pub fn list_mods(instance_name: String) -> Vec<ModEntry> {
    if valid_instance_name(&instance_name).is_err() {
        return vec![];
    }
    let dir = mods_dir(&instance_name);
    if !dir.exists() {
        return vec![];
    }
    let index = read_index(&instance_name);
    let mut out: Vec<ModEntry> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let enabled = if name.ends_with(".jar") {
                        true
                    } else if name.ends_with(".jar.disabled") {
                        false
                    } else {
                        return None;
                    };
                    // A hand-disabled jar keeps working: fall back to the
                    // sibling key with/without the ".disabled" suffix.
                    let meta = index
                        .get(&name)
                        .or_else(|| {
                            if enabled {
                                index.get(&format!("{name}.disabled"))
                            } else {
                                name.strip_suffix(".disabled")
                                    .and_then(|s| index.get(s))
                            }
                        })
                        .cloned()
                        .unwrap_or_default();
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    Some(ModEntry {
                        filename: name,
                        enabled,
                        size,
                        meta,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| {
        let an = if a.meta.name.is_empty() {
            &a.filename
        } else {
            &a.meta.name
        };
        let bn = if b.meta.name.is_empty() {
            &b.filename
        } else {
            &b.meta.name
        };
        an.to_lowercase().cmp(&bn.to_lowercase())
    });
    out
}

// ── enable / disable ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_mod_enabled(
    instance_name: String,
    filename: String,
    enabled: bool,
) -> Result<String, String> {
    valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
        return Err("Bad file name".to_string());
    }
    // Idempotent: asking for the current state just returns the name.
    let target = if enabled {
        match filename.strip_suffix(".disabled") {
            Some(s) if s.ends_with(".jar") => s.to_string(),
            Some(_) => return Err("Only .jar mods can be toggled".to_string()),
            None => return Ok(filename),
        }
    } else {
        if filename.ends_with(".disabled") {
            return Ok(filename);
        }
        if !filename.ends_with(".jar") {
            return Err("Only .jar mods can be toggled".to_string());
        }
        format!("{filename}.disabled")
    };
    let dir = mods_dir(&instance_name);
    if dir.join(&target).exists() {
        return Err(format!("{target} already exists"));
    }
    std::fs::rename(dir.join(&filename), dir.join(&target))
        .map_err(|e| format!("Rename failed: {e}"))?;
    let mut index = read_index(&instance_name);
    if let Some(meta) = index.remove(&filename) {
        index.insert(target.clone(), meta);
        let _ = write_index(&instance_name, &index);
    }
    Ok(target)
}

// ── install ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn install_mod_file(
    instance_name: String,
    download_url: String,
    file_name: String,
    replace_existing: Option<String>,
    meta: ModMeta,
) -> Result<String, String> {
    valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    if !mod_host_allowed(&download_url) {
        return Err(format!(
            "Refusing mod download from untrusted host: {download_url}"
        ));
    }
    let safe: String = file_name
        .chars()
        .filter(|&c| c != '/' && c != '\\' && c != '\0')
        .collect();
    if !safe.ends_with(".jar") {
        return Err("Only .jar files are supported".to_string());
    }
    let dir = mods_dir(&instance_name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let resp = http()?
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    // Download to a sidecar first so a failed install never leaves a
    // half-written jar that Minecraft would try (and fail) to load.
    let tmp = dir.join(format!(".{safe}.part"));
    let mut file = std::fs::File::create(&tmp).map_err(|e| format!("Write failed: {e}"))?;
    let mut resp = resp;
    let dl: Result<(), String> = async {
        use std::io::Write;
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| format!("Download interrupted: {e}"))?
        {
            file.write_all(&chunk)
                .map_err(|e| format!("Write failed: {e}"))?;
        }
        Ok(())
    }
    .await;
    if dl.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    dl?;
    drop(file);

    // Updates replace the old jar (different file name per version, usually).
    if let Some(old) = replace_existing.as_ref() {
        let old_safe: String = old
            .chars()
            .filter(|&c| c != '/' && c != '\\' && c != '\0')
            .collect();
        if old_safe != safe {
            let _ = std::fs::remove_file(dir.join(&old_safe));
            let mut index = read_index(&instance_name);
            index.remove(&old_safe);
            let _ = write_index(&instance_name, &index);
        }
    }
    std::fs::rename(&tmp, dir.join(&safe)).map_err(|e| format!("Install failed: {e}"))?;

    let mut index = read_index(&instance_name);
    index.insert(safe.clone(), meta);
    write_index(&instance_name, &index)?;
    Ok(safe)
}

// ── update checks ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_mod_updates(
    instance_name: String,
    mc_version: String,
    loader: String,
    cf_key: String,
) -> Result<Vec<ModUpdate>, String> {
    valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let dir = mods_dir(&instance_name);
    // Only indexed mods whose file is still on disk. CurseForge entries are
    // skipped without a key — Modrinth results still come back.
    let jobs: Vec<(String, ModMeta)> = read_index(&instance_name)
        .into_iter()
        .filter(|(f, m)| {
            (m.source == "modrinth"
                || (m.source == "curseforge" && !cf_key.trim().is_empty()))
                && dir.join(f).exists()
        })
        .collect();
    // The turbofish pins the task error to String: without it the `?` on
    // `latest_mod_file` leaves JoinSet's error type ambiguous (E0282).
    let mut set = tokio::task::JoinSet::<Result<Option<ModUpdate>, String>>::new();
    for (filename, meta) in jobs {
        let mc = mc_version.clone();
        let ld = loader.clone();
        let key = cf_key.clone();
        set.spawn(async move {
            let latest = latest_mod_file(&meta.source, &meta.project_id, &mc, &ld, &key).await?;
            if latest.version_id != meta.version_id && !latest.version_id.is_empty() {
                Ok(Some(ModUpdate {
                    filename,
                    name: meta.name.clone(),
                    current: meta.version_number.clone(),
                    latest: if latest.version_number.is_empty() {
                        latest.name.clone()
                    } else {
                        latest.version_number.clone()
                    },
                    version_id: latest.version_id.clone(),
                    file_name: latest.file_name.clone(),
                    download_url: latest.download_url.clone(),
                    size: latest.size,
                    source: meta.source.clone(),
                    project_id: meta.project_id.clone(),
                    author: meta.author.clone(),
                    icon_url: meta.icon_url.clone(),
                }))
            } else {
                Ok(None)
            }
        });
    }
    // One rotten mod (deleted upstream, flaky network) must not fail the
    // whole check — collect what succeeded.
    let mut out: Vec<ModUpdate> = Vec::new();
    while let Some(r) = set.join_next().await {
        if let Ok(Ok(Some(u))) = r {
            out.push(u);
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_filter_maps_all_loaders() {
        assert_eq!(loader_filter("fabric"), (Some("fabric"), Some(4)));
        assert_eq!(loader_filter("quilt"), (Some("quilt"), Some(5)));
        assert_eq!(loader_filter("forge"), (Some("forge"), Some(1)));
        assert_eq!(loader_filter("neoforge"), (Some("neoforge"), Some(6)));
        assert_eq!(loader_filter("vanilla"), (None, None));
        assert_eq!(loader_filter(""), (None, None));
    }
    #[test]
    fn mc_filter_ignores_empty_and_latest() {
        assert_eq!(mc_filter("1.20.1"), Some("1.20.1"));
        assert_eq!(mc_filter(""), None);
        assert_eq!(mc_filter("latest"), None);
    }
}
