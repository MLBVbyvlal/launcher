mod launcher;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};

// ── Microsoft auth ────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct MsAccount {
    username: String,
    uuid: String,
    access_token: String,
    refresh_token: String,
}

#[tauri::command]
async fn microsoft_login(app: tauri::AppHandle) -> Result<MsAccount, String> {
    const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
    const CLIENT_ID:    &str = "00000000402b5328";

    let auth_url = url::Url::parse(&format!(
        "https://login.live.com/oauth20_authorize.srf\
         ?client_id={CLIENT_ID}\
         &response_type=code\
         &scope=service%3A%3Auser.auth.xboxlive.com%3A%3AMBI_SSL\
         &redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf\
         &prompt=login"
    )).map_err(|e| format!("URL: {e}"))?;

    // Shared state between on_navigation callback and async polling loop
    let code_slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let closed     = Arc::new(AtomicBool::new(false));

    let code_nav   = code_slot.clone();
    let closed_ev  = closed.clone();

    // Embedded WebView window — Microsoft login renders here
    let win = tauri::WebviewWindowBuilder::new(
        &app, "ms-auth",
        tauri::WebviewUrl::External(auth_url),
    )
    .title("Sign in with Microsoft — MLBV")
    .inner_size(480.0, 660.0)
    .center()
    .on_navigation(move |url| {
        // Intercept the final redirect to oauth20_desktop.srf
        if url.host_str() == Some("login.live.com")
            && url.path() == "/oauth20_desktop.srf"
        {
            // query_pairs() properly URL-decodes the code value
            let code = url.query_pairs()
                .find(|(k, _)| k == "code")
                .map(|(_, v)| v.into_owned());
            if let Some(c) = code {
                *code_nav.lock().unwrap() = Some(c);
            }
            return false; // block the blank redirect page from loading
        }
        true
    })
    .build()
    .map_err(|e| format!("Auth window: {e}"))?;

    // Detect window close (user cancelled)
    win.on_window_event({
        let c = closed_ev.clone();
        move |ev| {
            if matches!(ev, tauri::WindowEvent::Destroyed
                          | tauri::WindowEvent::CloseRequested { .. }) {
                c.store(true, Ordering::Relaxed);
            }
        }
    });

    // Poll until we have the code or the window is closed
    let start = std::time::Instant::now();
    let auth_code = loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        {
            let g = code_slot.lock().unwrap();
            if let Some(c) = g.as_ref() { break c.clone(); }
        }
        if closed.load(Ordering::Relaxed) {
            return Err("Sign-in cancelled.".to_string());
        }
        if start.elapsed().as_secs() > 300 {
            let _ = win.close();
            return Err("Login timed out after 5 minutes.".to_string());
        }
    };
    let _ = win.close();

    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;

    // Exchange auth code → MS access token
    let ms: serde_json::Value = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", CLIENT_ID),
            ("code", auth_code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", REDIRECT_URI),
        ])
        .send().await.map_err(|e| format!("MS token: {e}"))?
        .json().await.map_err(|e| format!("MS token parse: {e}"))?;

    let ms_token = ms["access_token"].as_str()
        .ok_or_else(|| format!("MS auth failed: {ms}"))?
        .to_string();
    let refresh_token = ms["refresh_token"].as_str().unwrap_or("").to_string();

    // Xbox Live token
    let xbl: serde_json::Value = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": ms_token,
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .send().await.map_err(|e| format!("Xbox Live: {e}"))?
        .json().await.map_err(|e| format!("XBL parse: {e}"))?;

    let xbl_token = xbl["Token"].as_str().ok_or("Xbox Live auth failed")?;
    let uhs = xbl["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or("No UHS in XBL")?;

    // 6. XSTS token
    let xsts: serde_json::Value = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send().await.map_err(|e| format!("XSTS: {e}"))?
        .json().await.map_err(|e| format!("XSTS parse: {e}"))?;

    if let Some(xerr) = xsts["XErr"].as_u64() {
        return Err(match xerr {
            2148916238 => "Parental consent required for this Xbox account.".to_string(),
            2148916235 => "Xbox Live is not available in your region.".to_string(),
            2148916233 => "No Xbox account — create one at xbox.com first.".to_string(),
            _ => format!("Xbox error {xerr}"),
        });
    }
    let xsts_token = xsts["Token"].as_str().ok_or("XSTS auth failed")?;

    // 7. Minecraft token
    let mc_auth: serde_json::Value = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{xsts_token}"),
        }))
        .send().await.map_err(|e| format!("Minecraft auth: {e}"))?
        .json().await.map_err(|e| format!("MC auth parse: {e}"))?;

    let mc_token = mc_auth["access_token"].as_str()
        .ok_or("Minecraft auth failed — account may not own Minecraft Java Edition")?
        .to_string();

    // 8. Minecraft profile (UUID + username)
    let profile: serde_json::Value = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Authorization", format!("Bearer {mc_token}"))
        .send().await.map_err(|e| format!("Profile fetch: {e}"))?
        .json().await.map_err(|e| format!("Profile parse: {e}"))?;

    if profile["error"].is_string() {
        return Err("This account does not own Minecraft Java Edition.".to_string());
    }

    let uuid     = profile["id"].as_str().ok_or("No UUID in profile")?.to_string();
    let username = profile["name"].as_str().ok_or("No name in profile")?.to_string();

    Ok(MsAccount { username, uuid, access_token: mc_token, refresh_token })
}

// ── LiquidBounce version list ─────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
struct LbBranches {
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
async fn get_lb_branches() -> Result<Vec<String>, String> {
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
async fn get_lb_versions(branch: String) -> Result<Vec<LbBuild>, String> {
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
fn check_version_installed(version_id: String) -> bool {
    let dir = launcher::mc_dir();
    let jar = dir
        .join("versions")
        .join(&version_id)
        .join(format!("{version_id}.jar"));
    jar.exists()
}

#[tauri::command]
fn get_game_dir() -> String {
    launcher::shared_data_dir().to_string_lossy().into_owned()
}

#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    version_id: String,
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<(), String> {
    launcher::run(app, version_id, instance_name, username, uuid, offline, access_token, concurrent_downloads, max_ram_mb)
        .await
        .map_err(|e| format!("{:#}", e))
}

#[tauri::command]
async fn launch_lb_game(
    app: tauri::AppHandle,
    build_id: u32,
    mc_version: String,
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<(), String> {
    launcher::run_lb(app, build_id, mc_version, instance_name, username, uuid, offline, access_token, concurrent_downloads, max_ram_mb)
        .await
        .map_err(|e| format!("{:#}", e))
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
async fn stop_game(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<launcher::GameState>();
    let mut guard = state.child.lock().map_err(|_| "lock error")?;
    if let Some(ref mut child) = *guard {
        child.kill().map_err(|e| format!("Kill failed: {e}"))?;
        *guard = None;
    }
    drop(guard);
    let _ = app.emit("game-running", false);
    Ok(())
}

#[tauri::command]
fn read_instance_log(instance_name: String) -> String {
    let log_path = launcher::instances_dir()
        .join(&instance_name)
        .join("logs")
        .join("latest.log");
    std::fs::read_to_string(&log_path).unwrap_or_default()
}

#[tauri::command]
fn open_instance_logs_folder(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let logs_dir = launcher::instances_dir().join(&instance_name).join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    app.opener()
       .open_path(logs_dir.to_string_lossy().as_ref(), None::<&str>)
       .map_err(|e| e.to_string())
}

#[tauri::command]
fn reinstall_instance(instance_name: String, full_wipe: bool) -> Result<(), String> {
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

// ── Update check ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct ReleaseInfo {
    version:  String,
    tag_name: String,
    body:     String,
    html_url: String,
    asset_url: String,   // direct .exe download URL; empty if no asset found
}

fn parse_semver(v: &str) -> (u64, u64, u64) {
    let v = v.trim_start_matches('v');
    let mut parts = v.splitn(3, '.').map(|p| p.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

#[tauri::command]
async fn check_for_update() -> Result<Option<ReleaseInfo>, String> {
    let current = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;
    let releases: Vec<serde_json::Value> = client
        .get("https://api.github.com/repos/MLBVbyvlal/launcher/releases")
        .header("Accept", "application/vnd.github+json")
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    // First non-draft published release (includes pre-releases/betas)
    let release = releases.iter()
        .find(|r| !r["draft"].as_bool().unwrap_or(false))
        .ok_or_else(|| "No releases found".to_string())?;
    let tag = release["tag_name"].as_str().unwrap_or("").to_string();
    let ver = tag.trim_start_matches('v');
    if parse_semver(ver) <= parse_semver(current) {
        return Ok(None);
    }
    let asset_url = release["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|asset| {
            asset["name"].as_str().map(|n| n.ends_with(".exe")).unwrap_or(false)
        }))
        .and_then(|a| a["browser_download_url"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(Some(ReleaseInfo {
        version:  ver.to_string(),
        tag_name: tag,
        body:     release["body"].as_str().unwrap_or("").to_string(),
        html_url: release["html_url"].as_str().unwrap_or("").to_string(),
        asset_url,
    }))
}

// Download the update installer to %TEMP% and emit progress events
#[tauri::command]
async fn download_update(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let tmp_path = std::env::temp_dir().join("mlbv-update.exe");
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Cannot create temp file: {e}"))?;
    let mut downloaded: u64 = 0;
    let mut resp = resp;

    while let Some(chunk) = resp.chunk().await
        .map_err(|e| format!("Download interrupted: {e}"))?
    {
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| format!("Write failed: {e}"))?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let pct = downloaded as f32 / total as f32 * 100.0;
            let _ = app.emit("update-progress", serde_json::json!({ "percent": pct }));
        }
    }
    Ok(())
}

// Run the downloaded installer with the real install path, then exit
#[tauri::command]
fn apply_update(app: tauri::AppHandle, new_version: String) -> Result<(), String> {
    let tmp_path = std::env::temp_dir().join("mlbv-update.exe");
    if !tmp_path.exists() {
        return Err("Update installer not found".to_string());
    }

    let install_dir = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or_else(|| "Cannot determine install directory".to_string())?
        .to_path_buf();

    // Write the new version so the next launch can show "Updated to vX" toast
    let marker = std::env::temp_dir().join("mlbv-just-updated.txt");
    let _ = std::fs::write(&marker, &new_version);

    // NSIS silent install: /S = silent, /D= = destination (must be last, no quotes)
    std::process::Command::new(&tmp_path)
        .arg("/S")
        .arg(format!("/D={}", install_dir.to_string_lossy()))
        .spawn()
        .map_err(|e| format!("Failed to start installer: {e}"))?;

    std::thread::sleep(std::time::Duration::from_millis(400));
    app.exit(0);
    Ok(())
}

// Check if we just updated — returns old version string, or empty if not
#[tauri::command]
fn get_just_updated() -> String {
    let marker = std::env::temp_dir().join("mlbv-just-updated.txt");
    if !marker.exists() { return String::new(); }
    let ver = std::fs::read_to_string(&marker).unwrap_or_default();
    let _ = std::fs::remove_file(&marker);
    ver.trim().to_string()
}

#[tauri::command]
fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_download(app: tauri::AppHandle) {
    use std::sync::atomic::Ordering;
    app.state::<launcher::GameState>().cancel_dl.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn pause_download(app: tauri::AppHandle) -> bool {
    use std::sync::atomic::Ordering;
    let state = app.state::<launcher::GameState>();
    let new_val = !state.pause_dl.load(Ordering::Relaxed);
    state.pause_dl.store(new_val, Ordering::Relaxed);
    new_val // returns true = now paused
}

#[tauri::command]
async fn reset_all_data() -> Result<(), String> {
    let base = launcher::mlbv_base();
    if base.exists() {
        std::fs::remove_dir_all(&base).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(launcher::GameState::new())
        .invoke_handler(tauri::generate_handler![
            check_version_installed,
            get_game_dir,
            launch_game,
            launch_lb_game,
            get_lb_branches,
            get_lb_versions,
            microsoft_login,
            scan_java,
            download_java,
            stop_game,
            cancel_download,
            pause_download,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
