//! Instance-scoped commands: scan/metadata, mods folder ops, LiquidBounce
//! version lists and config install, loader version lists.
use crate::{launcher, mods};

// ── LiquidBounce version list ─────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct LbBranches {
    #[serde(rename = "defaultBranch")]
    default_branch: String,
    branches: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LbBuild {
    #[serde(default)]
    pub build_id: u32,
    #[serde(default)]
    pub lb_version: String,
    #[serde(default)]
    pub mc_version: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub branch: String,
}

#[tauri::command]
pub(crate) async fn get_lb_branches() -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let text = client
        .get("https://api.liquidbounce.net/api/v1/version/branches")
        .send().await.map_err(|e| format!("branches request: {e}"))?
        .text().await.map_err(|e| format!("branches read: {e}"))?;

    let branches: LbBranches = serde_json::from_str(&text)
        .map_err(|e| format!("branches parse: {e}"))?;

    Ok(branches.branches)
}

#[tauri::command]
pub(crate) async fn get_lb_versions(branch: String) -> Result<Vec<LbBuild>, String> {
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let builds_text = client
        .get(format!("https://api.liquidbounce.net/api/v1/version/builds/{branch}/release"))
        .send().await.map_err(|e| format!("builds request: {e}"))?
        .text().await.map_err(|e| format!("builds read: {e}"))?;

    let builds: Vec<LbBuild> = serde_json::from_str(&builds_text)
        .map_err(|e| format!("builds parse: {e} — body prefix: {}", &builds_text[..builds_text.len().min(300)]))?;

    Ok(builds)
}

// ── LB config install ────────────────────────────────────────────────────────

#[tauri::command]
pub(crate) async fn install_lb_config(json_url: String, instance_name: String, file_name: String) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    let bytes = client
        .get(&json_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    let instance_dir = launcher::instances_dir().join(&instance_name);
    // Auto-detect LiquidBounce configs folder (case-insensitive, with or without dot prefix)
    let configs_dir = ["LiquidBounce", "liquidbounce", ".liquidbounce"]
        .iter()
        .map(|f| instance_dir.join(f).join("configs"))
        .find(|p| p.exists())
        .unwrap_or_else(|| instance_dir.join("LiquidBounce").join("configs"));
    std::fs::create_dir_all(&configs_dir).map_err(|e| e.to_string())?;
    std::fs::write(configs_dir.join(&file_name), bytes).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
pub(crate) struct LbInstallTarget {
    name: String,
}

#[tauri::command]
pub(crate) fn get_lb_installable_instances() -> Vec<LbInstallTarget> {
    let base = launcher::instances_dir();
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(&base) else { return result; };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().into_owned();
        let has_lb = ["LiquidBounce", "liquidbounce", ".liquidbounce"]
            .iter()
            .any(|f| path.join(f).is_dir());
        if has_lb {
            result.push(LbInstallTarget { name });
        }
    }
    result
}

// ── Instance auto-detect ─────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub(crate) struct FoundInstance {
    name: String,
    instance_type: String,
    mc_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
    build_id: Option<u32>,
}

#[tauri::command]
pub(crate) fn scan_instances() -> Vec<FoundInstance> {
    let base = launcher::instances_dir();
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(&base) else { return found; };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().into_owned();
        // Try reading .mlbv-instance.json first
        let meta_path = path.join(".mlbv-instance.json");
        if let Ok(text) = std::fs::read_to_string(&meta_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                found.push(FoundInstance {
                    name: v["name"].as_str().unwrap_or(&name).to_string(),
                    instance_type: v["type"].as_str().unwrap_or("mc").to_string(),
                    mc_version: v["mcVersion"].as_str().map(|s| s.to_string()),
                    loader: v["loader"].as_str().map(|s| s.to_string()),
                    loader_version: v["loaderVersion"].as_str().map(|s| s.to_string()),
                    build_id: v["buildId"].as_u64().map(|n| n as u32),
                });
                continue;
            }
        }
        // Guess type from filesystem
        let is_lb = path.join(".liquidbounce").is_dir();
        found.push(FoundInstance {
            name: name.clone(),
            instance_type: if is_lb { "lb".to_string() } else { "mc".to_string() },
            mc_version: None,
            loader: None,
            loader_version: None,
            build_id: None,
        });
    }
    found
}

// ── Instance metadata save/delete ────────────────────────────────────────────

#[tauri::command]
pub(crate) fn save_instance_metadata(
    instance_name: String,
    instance_type: String,
    mc_version: String,
    loader: String,
    loader_version: String,
    build_id: Option<u32>,
) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let meta = serde_json::json!({
        "name": instance_name,
        "type": instance_type,
        "mcVersion": mc_version,
        "loader": loader,
        "loaderVersion": loader_version,
        "buildId": build_id,
    });
    let path = launcher::instances_dir()
        .join(&instance_name)
        .join(".mlbv-instance.json");
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&path, meta.to_string()).map_err(|e| e.to_string())
}

/// Rename the on-disk instance directory so a renamed instance keeps its
/// saves/mods (without this, renaming only changed the UI label and the next
/// launch silently started from a fresh, empty directory).
#[tauri::command]
pub(crate) fn rename_instance_data(old_name: String, new_name: String) -> Result<(), String> {
    launcher::valid_instance_name(&old_name).map_err(|e| e.to_string())?;
    launcher::valid_instance_name(&new_name).map_err(|e| e.to_string())?;
    let base = launcher::instances_dir();
    let old_dir = base.join(&old_name);
    if !old_dir.exists() { return Ok(()); }
    let new_dir = base.join(&new_name);
    if new_dir.exists() {
        return Err("Target instance directory already exists".to_string());
    }
    std::fs::rename(&old_dir, &new_dir).map_err(|e| format!("Rename failed: {e}"))
}

#[tauri::command]
pub(crate) fn delete_mods(instance_name: String, filenames: Vec<String>) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let mods_dir = launcher::instances_dir().join(&instance_name).join("mods");
    for filename in &filenames {
        if filename.contains('/') || filename.contains('\\') { continue; }
        let path = mods_dir.join(filename);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("Cannot delete {filename}: {e}"))?;
        }
    }
    mods::forget_mods(&instance_name, &filenames);
    Ok(())
}

#[tauri::command]
pub(crate) fn add_mod_file(instance_name: String, filename: String, data: Vec<u8>) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let safe: String = filename.chars().filter(|&c| c != '/' && c != '\\' && c != '\0').collect();
    if !safe.ends_with(".jar") { return Err("Only .jar files are supported".to_string()); }
    let mods_dir = launcher::instances_dir().join(&instance_name).join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    std::fs::write(mods_dir.join(&safe), &data).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn open_mods_folder(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let mods_dir = launcher::instances_dir().join(&instance_name).join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    app.opener()
       .open_path(mods_dir.to_string_lossy().as_ref(), None::<&str>)
       .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn delete_instance_data(instance_name: String) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let inst_dir = launcher::instances_dir().join(&instance_name);
    if inst_dir.exists() {
        std::fs::remove_dir_all(&inst_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(serde::Serialize, Clone)]
pub(crate) struct LoaderVersionInfo {
    version: String,
    stable:  bool,
    latest:  bool,
}

fn is_unstable_ver(v: &str) -> bool {
    let l = v.to_lowercase();
    l.contains("beta") || l.contains("alpha") || l.contains("-rc") || l.contains(".rc")
        || l.contains("pre") || l.contains("snapshot") || l.contains("-b.")
}

fn tag_latest(mut items: Vec<LoaderVersionInfo>) -> Vec<LoaderVersionInfo> {
    if let Some(first_stable) = items.iter_mut().find(|i| i.stable) {
        first_stable.latest = true;
    }
    items
}

/// Every `<version>x</version>` from a maven-metadata.xml, whether the file
/// is pretty-printed (one version per line) or minified (all on one line).
/// Token-based on purpose: a line-based parser silently returns nothing for
/// the minified shape.
fn parse_maven_versions(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<version>") {
        rest = &rest[start + "<version>".len()..];
        let Some(end) = rest.find("</version>") else { break };
        out.push(rest[..end].trim().to_string());
        rest = &rest[end + "</version>".len()..];
    }
    out
}

#[tauri::command]
pub(crate) async fn get_loader_versions(mc_ver: String, loader: String) -> Result<Vec<LoaderVersionInfo>, String> {
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    match loader.as_str() {
        "fabric" => {
            let url = format!("https://meta.fabricmc.net/v2/versions/loader/{}", mc_ver);
            let data: serde_json::Value = client.get(&url).send().await
                .map_err(|e| e.to_string())?
                .json().await.map_err(|e| e.to_string())?;
            let items = data.as_array().map(|arr| arr.iter()
                .filter_map(|v| {
                    let version = v["loader"]["version"].as_str()?.to_string();
                    let api_stable = v["loader"]["stable"].as_bool().unwrap_or(false);
                    let stable  = api_stable || !is_unstable_ver(&version);
                    Some(LoaderVersionInfo { version, stable, latest: false })
                })
                .collect::<Vec<_>>()
            ).unwrap_or_default();
            Ok(tag_latest(items))
        }
        "quilt" => {
            let url = format!("https://meta.quiltmc.org/v3/versions/loader/{}", mc_ver);
            let data: serde_json::Value = client.get(&url).send().await
                .map_err(|e| e.to_string())?
                .json().await.map_err(|e| e.to_string())?;
            let items = data.as_array().map(|arr| arr.iter()
                .filter_map(|v| {
                    let version = v["loader"]["version"].as_str()?.to_string();
                    let stable  = !is_unstable_ver(&version);
                    Some(LoaderVersionInfo { version, stable, latest: false })
                })
                .collect::<Vec<_>>()
            ).unwrap_or_default();
            Ok(tag_latest(items))
        }
        "forge" => {
            let xml = client
                .get("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")
                .send().await.map_err(|e| e.to_string())?
                .text().await.map_err(|e| e.to_string())?;
            let prefix = format!("{}-", mc_ver);
            let items: Vec<LoaderVersionInfo> = parse_maven_versions(&xml).into_iter()
                .filter(|v| v.starts_with(&prefix))
                .map(|version| {
                    let stable = !is_unstable_ver(&version);
                    LoaderVersionInfo { version, stable, latest: false }
                })
                .rev().take(30).collect();
            Ok(tag_latest(items))
        }
        "neoforge" => {
            let xml = client
                .get("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml")
                .send().await.map_err(|e| e.to_string())?
                .text().await.map_err(|e| e.to_string())?;
            let parts: Vec<&str> = mc_ver.split('.').collect();
            let prefix = match parts.as_slice() {
                [_, b, c] => format!("{}.{}.", b, c),
                [_, b] => format!("{}.", b),
                _ => return Err(format!("Invalid MC version: {mc_ver}")),
            };
            let items: Vec<LoaderVersionInfo> = parse_maven_versions(&xml).into_iter()
                .filter(|v| v.starts_with(&prefix))
                .map(|version| {
                    let stable = !is_unstable_ver(&version);
                    LoaderVersionInfo { version, stable, latest: false }
                })
                .rev().take(30).collect();
            Ok(tag_latest(items))
        }
        _ => Err(format!("Unknown loader: {loader}")),
    }
}


// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_unstable_ver_spots_prerelease_markers() {
        for stable in ["0.15.11", "0.26.0", "1.20.1-47.3.11", "21.1.172", "52.0.12"] {
            assert!(!is_unstable_ver(stable), "{stable} should count as stable");
        }
        for unstable in [
            "0.16.0-beta.1",
            "2.0.0-alpha.3",
            "1.0.0-rc1",
            "1.0.0-rc.2",
            "1.0.0-pre1",
            "1.21-snapshot-5",
            "1.0.0-b.7",
        ] {
            assert!(is_unstable_ver(unstable), "{unstable} should count as unstable");
        }
    }
    #[test]
    fn tag_latest_marks_first_stable_only() {
        let mk = |version: &str, stable: bool| LoaderVersionInfo {
            version: version.to_string(),
            stable,
            latest: false,
        };
        let tagged = tag_latest(vec![mk("2.0-beta", false), mk("1.9", true), mk("1.8", true)]);
        assert_eq!(tagged.iter().filter(|i| i.latest).count(), 1);
        assert_eq!(tagged[1].version, "1.9");
        assert!(!tagged[0].latest && !tagged[2].latest);
        assert!(tag_latest(vec![]).is_empty());
        let none_stable = tag_latest(vec![mk("2.0-beta", false)]);
        assert!(!none_stable[0].latest);
    }
    #[test]
    fn parse_maven_versions_handles_pretty_and_minified_xml() {
        let pretty = "<metadata>\n  <versions>\n    <version>1.20.1-47.0.1</version>\n    <version>1.20.1-47.3.11</version>\n  </versions>\n</metadata>";
        assert_eq!(
            parse_maven_versions(pretty),
            vec!["1.20.1-47.0.1", "1.20.1-47.3.11"]
        );
        let minified = "<metadata><versions><version>21.1.172</version><version>21.1.175</version></versions></metadata>";
        assert_eq!(parse_maven_versions(minified), vec!["21.1.172", "21.1.175"]);
        assert!(parse_maven_versions("no versions here").is_empty());
        assert!(parse_maven_versions("").is_empty());
        assert!(parse_maven_versions("<version>unterminated").is_empty());
    }
}
