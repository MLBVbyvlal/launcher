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
use std::collections::HashMap;
use std::path::PathBuf;

use super::launcher::{instances_dir, valid_instance_name};

const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const CURSEFORGE_API: &str = "https://api.curseforge.com/v1";
// Modrinth rejects requests without a descriptive User-Agent.
const UA: &str = "MLBV/0.0.5 (github.com/MLBVbyvlal/launcher)";

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(|e| e.to_string())
}

// ── shared shapes ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ModHit {
    id: String,
    slug: String,
    name: String,
    author: String,
    summary: String,
    icon_url: String,
    downloads: u64,
    updated: String,
}

#[derive(serde::Serialize)]
pub struct ModSearchResult {
    items: Vec<ModHit>,
    total: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct ModFile {
    version_id: String,
    name: String,
    version_number: String,
    mc_versions: Vec<String>,
    loaders: Vec<String>,
    /// "release" | "beta" | "alpha".
    release_type: String,
    file_name: String,
    download_url: String,
    size: u64,
    published: String,
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
    filename: String,
    enabled: bool,
    size: u64,
    #[serde(flatten)]
    meta: ModMeta,
}

#[derive(serde::Serialize)]
pub struct ModUpdate {
    filename: String,
    name: String,
    current: String,
    latest: String,
    version_id: String,
    file_name: String,
    download_url: String,
    size: u64,
    source: String,
    project_id: String,
    author: String,
    icon_url: String,
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

async fn search_modrinth(
    query: &str,
    mc: &str,
    loader: &str,
    limit: u32,
    offset: u32,
) -> Result<ModSearchResult, String> {
    let mut facets: Vec<Vec<String>> = vec![vec!["project_type:mod".to_string()]];
    if let Some(mc) = mc_filter(mc) {
        facets.push(vec![format!("versions:{mc}")]);
    }
    if let (Some(lf), _) = loader_filter(loader) {
        facets.push(vec![format!("categories:{lf}")]);
    }
    let facets_json = serde_json::to_string(&facets).map_err(|e| e.to_string())?;
    let v: serde_json::Value = http()?
        .get(format!("{MODRINTH_API}/search"))
        .query(&[
            ("query", query.to_string()),
            ("facets", facets_json),
            ("limit", limit.min(50).to_string()),
            ("offset", offset.to_string()),
            ("index", "relevance".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Modrinth search: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Modrinth search: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Modrinth search parse: {e}"))?;
    let items = v["hits"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|h| ModHit {
                    id: h["project_id"].as_str().unwrap_or("").to_string(),
                    slug: h["slug"].as_str().unwrap_or("").to_string(),
                    name: h["title"].as_str().unwrap_or("?").to_string(),
                    author: h["author"].as_str().unwrap_or("").to_string(),
                    summary: h["description"].as_str().unwrap_or("").to_string(),
                    icon_url: h["icon_url"].as_str().unwrap_or("").to_string(),
                    downloads: h["downloads"].as_u64().unwrap_or(0),
                    updated: h["date_modified"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(10)
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(ModSearchResult {
        items,
        total: v["total_hits"].as_u64().unwrap_or(0),
    })
}

async fn search_curseforge(
    query: &str,
    mc: &str,
    loader: &str,
    key: &str,
    limit: u32,
    offset: u32,
) -> Result<ModSearchResult, String> {
    if key.trim().is_empty() {
        return Err("CF_KEY_MISSING: no CurseForge API key set.".to_string());
    }
    let (_, cf_loader) = loader_filter(loader);
    let mut req = http()?
        .get(format!("{CURSEFORGE_API}/mods/search"))
        .header("x-api-key", key.trim())
        .query(&[
            ("gameId", "432".to_string()),
            ("classId", "6".to_string()),
            ("searchFilter", query.to_string()),
            ("pageSize", limit.min(50).to_string()),
            ("index", offset.to_string()),
            ("sortField", "2".to_string()),
            ("sortOrder", "desc".to_string()),
        ]);
    if let Some(mc) = mc_filter(mc) {
        req = req.query(&[("gameVersion", mc.to_string())]);
    }
    if let Some(l) = cf_loader {
        req = req.query(&[("modLoaderType", l.to_string())]);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("CurseForge search: {e}"))?;
    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err("CF_KEY_INVALID: CurseForge rejected the API key.".to_string());
    }
    let v: serde_json::Value = resp
        .error_for_status()
        .map_err(|e| format!("CurseForge search: {e}"))?
        .json()
        .await
        .map_err(|e| format!("CurseForge search parse: {e}"))?;
    let items = v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|m| ModHit {
                    id: m["id"].as_u64().unwrap_or(0).to_string(),
                    slug: m["slug"].as_str().unwrap_or("").to_string(),
                    name: m["name"].as_str().unwrap_or("?").to_string(),
                    author: m["authors"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|a| a["name"].as_str())
                        .unwrap_or("")
                        .to_string(),
                    summary: m["summary"].as_str().unwrap_or("").to_string(),
                    icon_url: m["logo"]["url"].as_str().unwrap_or("").to_string(),
                    downloads: m["downloadCount"].as_u64().unwrap_or(0),
                    updated: m["dateModified"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(10)
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(ModSearchResult {
        items,
        total: v["pagination"]["totalCount"].as_u64().unwrap_or(0),
    })
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

async fn mod_versions(
    source: &str,
    project_id: &str,
    mc: &str,
    loader: &str,
    key: &str,
) -> Result<Vec<ModFile>, String> {
    match source {
        "modrinth" => modrinth_versions(project_id, mc, loader).await,
        "curseforge" => curseforge_files(project_id, mc, loader, key).await,
        other => Err(format!("Unknown mod source: {other}")),
    }
}

async fn latest_mod_file(
    source: &str,
    project_id: &str,
    mc: &str,
    loader: &str,
    key: &str,
) -> Result<ModFile, String> {
    mod_versions(source, project_id, mc, loader, key)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| "No files for this version".to_string())
}

async fn modrinth_versions(
    project_id: &str,
    mc: &str,
    loader: &str,
) -> Result<Vec<ModFile>, String> {
    // The API takes JSON-encoded arrays, not repeated params.
    let mut req = http()?.get(format!("{MODRINTH_API}/project/{project_id}/version"));
    if let Some(mc) = mc_filter(mc) {
        req = req.query(&[("game_versions", format!("[\"{mc}\"]"))]);
    }
    if let (Some(lf), _) = loader_filter(loader) {
        req = req.query(&[("loaders", format!("[\"{lf}\"]"))]);
    }
    let v: serde_json::Value = req
        .send()
        .await
        .map_err(|e| format!("Modrinth versions: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Modrinth versions: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Modrinth versions parse: {e}"))?;
    Ok(v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|ver| {
                    let files = ver["files"].as_array()?;
                    let f = files
                        .iter()
                        .find(|f| f["primary"].as_bool().unwrap_or(false))
                        .or_else(|| files.first())?;
                    Some(ModFile {
                        version_id: ver["id"].as_str().unwrap_or("").to_string(),
                        name: ver["name"].as_str().unwrap_or("").to_string(),
                        version_number: ver["version_number"].as_str().unwrap_or("").to_string(),
                        mc_versions: ver["game_versions"]
                            .as_array()
                            .map(|g| {
                                g.iter()
                                    .filter_map(|g| g.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        loaders: ver["loaders"]
                            .as_array()
                            .map(|g| {
                                g.iter()
                                    .filter_map(|g| g.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        release_type: ver["version_type"].as_str().unwrap_or("release").to_string(),
                        file_name: f["filename"].as_str().unwrap_or("").to_string(),
                        download_url: f["url"].as_str().unwrap_or("").to_string(),
                        size: f["size"].as_u64().unwrap_or(0),
                        published: ver["date_published"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(10)
                            .collect(),
                    })
                })
                .filter(|f| !f.file_name.is_empty() && !f.download_url.is_empty())
                .collect()
        })
        .unwrap_or_default())
}

/// CurseForge file download URLs are sometimes null; the CDN layout is
/// `files/<id without last 3 digits>/<last 3 digits>/<name>` (same fallback
/// every third-party client uses).
fn cf_edge_url(file_id: u64, file_name: &str) -> String {
    let s = file_id.to_string();
    let cut = s.len().saturating_sub(3).max(1);
    format!(
        "https://edge.forgecdn.net/files/{}/{}/{file_name}",
        &s[..cut],
        &s[cut..]
    )
}

/// CurseForge mixes loader names into `gameVersions` — split them back out.
fn split_cf_game_versions(raw: &[String]) -> (Vec<String>, Vec<String>) {
    const LOADERS: [&str; 9] = [
        "Forge", "Fabric", "Quilt", "NeoForge", "Cauldron", "LiteLoader", "Rift", "OptiFine", "Bukkit",
    ];
    let mut mc = Vec::new();
    let mut loaders = Vec::new();
    for v in raw {
        if LOADERS.iter().any(|l| l.eq_ignore_ascii_case(v)) {
            loaders.push(v.clone());
        } else {
            mc.push(v.clone());
        }
    }
    (mc, loaders)
}

async fn curseforge_files(
    project_id: &str,
    mc: &str,
    loader: &str,
    key: &str,
) -> Result<Vec<ModFile>, String> {
    if key.trim().is_empty() {
        return Err("CF_KEY_MISSING: no CurseForge API key set.".to_string());
    }
    let (_, cf_loader) = loader_filter(loader);
    let mut req = http()?
        .get(format!("{CURSEFORGE_API}/mods/{project_id}/files"))
        .header("x-api-key", key.trim());
    if let Some(mc) = mc_filter(mc) {
        req = req.query(&[("gameVersion", mc.to_string())]);
    }
    if let Some(l) = cf_loader {
        req = req.query(&[("modLoaderType", l.to_string())]);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("CurseForge files: {e}"))?;
    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err("CF_KEY_INVALID: CurseForge rejected the API key.".to_string());
    }
    let v: serde_json::Value = resp
        .error_for_status()
        .map_err(|e| format!("CurseForge files: {e}"))?
        .json()
        .await
        .map_err(|e| format!("CurseForge files parse: {e}"))?;
    let mut out: Vec<ModFile> = v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| {
                    let file_id = f["id"].as_u64()?;
                    let file_name = f["fileName"].as_str().unwrap_or("").to_string();
                    if file_name.is_empty() {
                        return None;
                    }
                    let download_url = f["downloadUrl"]
                        .as_str()
                        .filter(|u| !u.is_empty())
                        .map(|u| u.to_string())
                        .unwrap_or_else(|| cf_edge_url(file_id, &file_name));
                    let raw_versions: Vec<String> = f["gameVersions"]
                        .as_array()
                        .map(|g| {
                            g.iter()
                                .filter_map(|g| g.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let (mut mc_versions, loaders) = split_cf_game_versions(&raw_versions);
                    if mc_versions.is_empty() {
                        if let Some(gv) = f["gameVersion"].as_str() {
                            mc_versions.push(gv.to_string());
                        }
                    }
                    let release_type = match f["releaseType"].as_u64().unwrap_or(1) {
                        2 => "beta",
                        3 => "alpha",
                        _ => "release",
                    }
                    .to_string();
                    Some(ModFile {
                        version_id: file_id.to_string(),
                        name: f["displayName"].as_str().unwrap_or("").to_string(),
                        version_number: String::new(),
                        mc_versions,
                        loaders,
                        release_type,
                        file_name,
                        download_url,
                        size: f["fileLength"].as_u64().unwrap_or(0),
                        published: f["fileDate"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(10)
                            .collect(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // Newest first (ISO dates sort lexicographically).
    out.sort_by(|a, b| b.published.cmp(&a.published));
    Ok(out)
}

// ── installed-mod index ───────────────────────────────────────────────────────

fn mods_dir(instance: &str) -> PathBuf {
    instances_dir().join(instance).join("mods")
}

fn index_path(instance: &str) -> PathBuf {
    mods_dir(instance).join(".mlbv-mods.json")
}

fn read_index(instance: &str) -> HashMap<String, ModMeta> {
    std::fs::read_to_string(index_path(instance))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_index(instance: &str, index: &HashMap<String, ModMeta>) -> Result<(), String> {
    let dir = mods_dir(instance);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
    std::fs::write(index_path(instance), text).map_err(|e| e.to_string())
}

/// Drop index entries (used when files are deleted outside this module).
pub(crate) fn forget_mods(instance: &str, filenames: &[String]) {
    if valid_instance_name(instance).is_err() {
        return;
    }
    let mut index = read_index(instance);
    let before = index.len();
    for f in filenames {
        index.remove(f);
    }
    if index.len() != before {
        let _ = write_index(instance, &index);
    }
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

/// Only mod-platform CDNs are trusted for jar downloads. Same reasoning as
/// the updater allowlist: the bytes end up executed, so the host matters.
fn mod_host_allowed(url: &str) -> bool {
    const TRUSTED_HOSTS: [&str; 4] = [
        "cdn.modrinth.com",
        "api.modrinth.com",
        "edge.forgecdn.net",
        "media.forgecdn.net",
    ];
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" {
        return false;
    }
    let Some(host) = parsed.host_str() else {
        return false;
    };
    TRUSTED_HOSTS.contains(&host.to_ascii_lowercase().as_str())
}

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
    let mut set = tokio::task::JoinSet::new();
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

    #[test]
    fn cf_edge_url_splits_file_id() {
        assert_eq!(
            cf_edge_url(1234567, "cool-mod-1.0.jar"),
            "https://edge.forgecdn.net/files/1234/567/cool-mod-1.0.jar"
        );
        assert_eq!(
            cf_edge_url(12345678, "m.jar"),
            "https://edge.forgecdn.net/files/12345/678/m.jar"
        );
    }

    #[test]
    fn split_cf_game_versions_separates_loaders() {
        let (mc, loaders) = split_cf_game_versions(&[
            "1.20.1".to_string(),
            "Forge".to_string(),
            "1.20".to_string(),
        ]);
        assert_eq!(mc, vec!["1.20.1", "1.20"]);
        assert_eq!(loaders, vec!["Forge"]);
    }

    #[test]
    fn mod_host_allowed_trusts_mod_cdns_only() {
        assert!(mod_host_allowed(
            "https://cdn.modrinth.com/data/abc/versions/x/mod.jar"
        ));
        assert!(mod_host_allowed(
            "https://edge.forgecdn.net/files/1234/567/mod.jar"
        ));
        assert!(mod_host_allowed(
            "https://EDGE.FORGECDN.NET/files/1/2/m.jar"
        ));
        for bad in [
            "https://evil.com/mod.jar",
            "https://cdn.modrinth.com.evil.com/m.jar",
            "http://cdn.modrinth.com/m.jar",
            "not a url",
            "",
        ] {
            assert!(!mod_host_allowed(bad), "{bad:?} must be rejected");
        }
    }
}
