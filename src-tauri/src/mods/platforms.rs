//! Modrinth and CurseForge HTTP clients: search, version listing, and the
//! CDN allow-list that `install_mod_file` enforces.
use super::*;

pub(super) async fn search_modrinth(
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

pub(super) async fn search_curseforge(
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

pub(super) async fn mod_versions(
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

pub(super) async fn latest_mod_file(
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

pub(super) async fn modrinth_versions(
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
pub(super) fn cf_edge_url(file_id: u64, file_name: &str) -> String {
    let s = file_id.to_string();
    let cut = s.len().saturating_sub(3).max(1);
    format!(
        "https://edge.forgecdn.net/files/{}/{}/{file_name}",
        &s[..cut],
        &s[cut..]
    )
}

/// CurseForge mixes loader names into `gameVersions` — split them back out.
pub(super) fn split_cf_game_versions(raw: &[String]) -> (Vec<String>, Vec<String>) {
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

pub(super) async fn curseforge_files(
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

// ── install: CDN allow-list ───────────────────────────────────────────────────

/// Only mod-platform CDNs are trusted for jar downloads. Same reasoning as
/// the updater allowlist: the bytes end up executed, so the host matters.
pub(super) fn mod_host_allowed(url: &str) -> bool {
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

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
