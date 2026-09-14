pub mod auth;
pub mod instances;
pub mod launcher;
pub mod migration;
pub mod mods;
pub mod updater;
pub mod vault;

use tauri::{Emitter, Manager};

use auth::*;
use instances::*;
use migration::*;
use updater::*;

#[tauri::command]
fn scan_java() -> Vec<serde_json::Value> {
    launcher::scan_java_installs()
        .into_iter()
        .map(|(major, path)| serde_json::json!({ "major": major, "path": path }))
        .collect()
}

#[tauri::command]
fn get_debug_info() -> serde_json::Value {
    let shared = launcher::shared_data_dir();
    let mc     = launcher::mc_dir();
    let java_dir = shared.join("java");
    let java_entries: Vec<String> = std::fs::read_dir(&java_dir)
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    let installs = launcher::scan_java_installs();
    serde_json::json!({
        "version":    env!("CARGO_PKG_VERSION"),
        "os":         std::env::consts::OS,
        "arch":       std::env::consts::ARCH,
        "shared_dir": shared.to_string_lossy(),
        "mc_dir":     mc.to_string_lossy(),
        "java_dir_exists": java_dir.exists(),
        "java_subdirs": java_entries,
        "java_installs": installs.iter().map(|(m, p)| format!("Java {m}: {p}")).collect::<Vec<_>>(),
    })
}

#[tauri::command]
fn get_game_dir() -> String {
    launcher::shared_data_dir().to_string_lossy().into_owned()
}

/// One launch command for every loader: `loader` selects the pipeline and
/// `loader_version` / `lb_build_id` carry what that loader needs.
/// `loader_version` is the Fabric/Quilt loader version or the full
/// Forge/NeoForge version; an empty string means "pick the newest stable".
#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    loader: launcher::Loader,
    mc_version: String,
    loader_version: String,
    lb_build_id: u32,
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    concurrent_downloads: u32,
    max_ram_mb: u32,
    java_path: String,
    jvm_args: String,
    min_ram_mb: u32,
) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    // The access token never crosses the bridge: it is read from the vault
    // here, refreshed first when the session is old.
    let access_token = if offline {
        "0".to_string()
    } else {
        let stored = app.state::<vault::Vault>().get(&uuid)
            .ok_or_else(|| "This Microsoft account is not signed in on this machine — sign in again.".to_string())?;
        if stored.is_stale() {
            match refresh_tokens(&stored.refresh_token).await {
                Ok((_, new_uuid, access, refresh)) => {
                    store_tokens(&app, &new_uuid, access.clone(), refresh)?;
                    access
                }
                // Keep the current token; if it is dead the server rejects
                // the login and the user signs in again.
                Err(_) => stored.access_token,
            }
        } else {
            stored.access_token
        }
    };
    launcher::launch(app, launcher::LaunchRequest {
        loader,
        mc_version,
        loader_version,
        lb_build_id,
        instance_name,
        username,
        uuid,
        offline,
        access_token,
        concurrent_downloads,
        max_ram_mb,
        java_path,
        jvm_args,
        min_ram_mb,
    })
    .await
    .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn download_java(app: tauri::AppHandle, major: u32) -> Result<(), String> {
    launcher::download_java_major(&app, major)
        .await
        .map_err(|e| {
            let _ = app.emit("java-progress", serde_json::json!({
                "major": major,
                "status": "error",
                "progress": 0.0,
                "message": e.to_string(),
            }));
            e.to_string()
        })
}

#[tauri::command]
async fn stop_game(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let state = app.state::<launcher::GameState>();
    let mut children = state.children.lock().map_err(|_| "lock error")?;
    let Some(mut child) = children.remove(&instance_name) else { return Ok(()) };
    drop(children);
    let killed = child.kill().map_err(|e| format!("Kill failed: {e}"));
    // The exit watcher is gone with the registry entry, so emit here.
    let _ = app.emit("game-running", serde_json::json!({
        "instance": instance_name,
        "running": false,
    }));
    killed?;
    Ok(())
}

#[tauri::command]
fn read_instance_log(instance_name: String) -> String {
    if launcher::valid_instance_name(&instance_name).is_err() { return String::new(); }
    let log_path = launcher::instances_dir()
        .join(&instance_name)
        .join("logs")
        .join("latest.log");
    std::fs::read_to_string(&log_path).unwrap_or_default()
}

#[tauri::command]
fn open_instance_logs_folder(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let logs_dir = launcher::instances_dir().join(&instance_name).join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    app.opener()
       .open_path(logs_dir.to_string_lossy().as_ref(), None::<&str>)
       .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_game_dir(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let game_dir = launcher::instances_dir().join(&instance_name);
    let _ = std::fs::create_dir_all(&game_dir);
    app.opener()
       .open_path(game_dir.to_string_lossy().as_ref(), None::<&str>)
       .map_err(|e| e.to_string())
}

#[tauri::command]
fn reinstall_instance(instance_name: String, full_wipe: bool) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let inst_dir = launcher::instances_dir().join(&instance_name);
    if !inst_dir.exists() { return Ok(()); }
    if full_wipe {
        std::fs::remove_dir_all(&inst_dir).map_err(|e| e.to_string())?;
    } else {
        let keep = ["saves", "screenshots", "resourcepacks", "options.txt",
                    "server-resource-packs", "servers.dat", "shaderpacks"];
        for entry in std::fs::read_dir(&inst_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !keep.iter().any(|k| *k == name_str.as_ref()) {
                if entry.path().is_dir() { let _ = std::fs::remove_dir_all(entry.path()); }
                else { let _ = std::fs::remove_file(entry.path()); }
            }
        }
    }
    Ok(())
}

/// Cancel the launch of one instance; other launches in flight are untouched.
#[tauri::command]
fn cancel_download(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    if let Some(ctl) = app.state::<launcher::GameState>().download_control(&instance_name) {
        ctl.cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// Toggle pause for one instance's launch. Returns true = now paused.
#[tauri::command]
fn pause_download(app: tauri::AppHandle, instance_name: String) -> Result<bool, String> {
    use std::sync::atomic::Ordering;
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let Some(ctl) = app.state::<launcher::GameState>().download_control(&instance_name) else {
        return Ok(false);
    };
    let new_val = !ctl.pause.load(Ordering::Relaxed);
    ctl.pause.store(new_val, Ordering::Relaxed);
    Ok(new_val)
}

/// Instances whose launch is still downloading/installing — lets the UI
/// re-attach its cards after a reload instead of showing an idle Play.
#[tauri::command]
fn active_downloads(app: tauri::AppHandle) -> Vec<String> {
    app.state::<launcher::GameState>().active_downloads()
}

#[tauri::command]
async fn reset_all_data() -> Result<(), String> {
    let base = launcher::mlbv_base();
    if base.exists() {
        std::fs::remove_dir_all(&base).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── JVM output streaming (Console tab polls this) ───────────────────────────────

#[derive(serde::Serialize)]
struct JvmPollResult {
    lines: Vec<String>,
    new_offset: usize,
    cleared: bool,
}

#[tauri::command]
fn poll_jvm_output(offset: usize, instance_name: String, app: tauri::AppHandle) -> JvmPollResult {
    if launcher::valid_instance_name(&instance_name).is_err() {
        return JvmPollResult { lines: vec![], new_offset: offset, cleared: false };
    }
    let state = app.state::<launcher::GameState>();
    let buffers = state.jvm_buffers.lock().unwrap();
    let Some(buffer) = buffers.get(&instance_name) else {
        // Nothing launched in this session: the console has no source yet.
        return JvmPollResult { lines: vec![], new_offset: 0, cleared: offset > 0 };
    };
    let lines = buffer.lock().unwrap();
    if offset > 0 && lines.is_empty() {
        return JvmPollResult { lines: vec![], new_offset: 0, cleared: true };
    }
    if offset >= lines.len() {
        return JvmPollResult { lines: vec![], new_offset: lines.len(), cleared: false };
    }
    let new_lines = lines[offset..].to_vec();
    let new_offset = lines.len();
    JvmPollResult { lines: new_lines, new_offset, cleared: false }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(launcher::GameState::new())
        .manage(vault::Vault::load())
        .invoke_handler(tauri::generate_handler![
            get_game_dir,
            launch_game,
            get_lb_branches,
            get_lb_versions,
            microsoft_login,
            refresh_ms_token,
            vault_has_account,
            vault_forget_account,
            rename_instance_data,
            scan_java,
            download_java,
            stop_game,
            cancel_download,
            pause_download,
            active_downloads,
            reset_all_data,
            read_instance_log,
            open_instance_logs_folder,
            reinstall_instance,
            check_for_update,
            download_update,
            apply_update,
            get_just_updated,
            open_url,
            get_debug_info,
            open_game_dir,
            poll_jvm_output,
            get_data_version,
            set_data_version,
            migration_scan,
            migration_clean,
            install_lb_config,
            get_lb_installable_instances,
            scan_instances,
            save_instance_metadata,
            mods::list_mods,
            mods::search_mods,
            mods::get_mod_versions,
            mods::set_mod_enabled,
            mods::install_mod_file,
            mods::check_mod_updates,
            delete_mods,
            add_mod_file,
            open_mods_folder,
            delete_instance_data,
            get_loader_versions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
