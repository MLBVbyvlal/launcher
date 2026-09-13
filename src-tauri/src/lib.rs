pub mod launcher;

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

/// Xbox Live → XSTS → Minecraft token chain, shared by the OAuth login flow
/// and the refresh-token flow. Returns (mc_token, uuid, username).
async fn ms_token_chain(client: &reqwest::Client, ms_token: &str) -> Result<(String, String, String), String> {
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

    // XSTS token
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

    // Minecraft token
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

    // Minecraft profile (UUID + username)
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

    Ok((mc_token, uuid, username))
}

/// Re-authenticate a stored Microsoft account from its refresh token.
/// Minecraft access tokens live ~24 h; without this the user has to repeat
/// the full login. Microsoft rotates refresh tokens — when a new one is
/// returned the frontend must persist it (the old one is then dead).
#[tauri::command]
async fn refresh_ms_token(refresh_token: String) -> Result<MsAccount, String> {
    const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
    const CLIENT_ID:    &str = "00000000402b5328";

    if refresh_token.trim().is_empty() {
        return Err("No refresh token stored — sign in again.".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;

    let ms: serde_json::Value = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
            ("redirect_uri", REDIRECT_URI),
        ])
        .send().await.map_err(|e| format!("MS refresh: {e}"))?
        .json().await.map_err(|e| format!("MS refresh parse: {e}"))?;

    let ms_token = ms["access_token"].as_str()
        .ok_or_else(|| format!("MS refresh failed: {ms}"))?
        .to_string();
    let new_refresh = ms["refresh_token"].as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| refresh_token.clone());

    let (mc_token, uuid, username) = ms_token_chain(&client, &ms_token).await?;
    Ok(MsAccount { username, uuid, access_token: mc_token, refresh_token: new_refresh })
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

    // Xbox → XSTS → Minecraft chain (shared with refresh_ms_token)
    let (mc_token, uuid, username) = ms_token_chain(&client, &ms_token).await?;

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
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
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

// ── Update check ─────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct ReleaseInfo {
    version:  String,
    tag_name: String,
    body:     String,
    html_url: String,
    asset_url: String,
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
    let asset_url = release["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|asset| {
            asset["name"].as_str().map(|n| n.ends_with(".exe")).unwrap_or(false)
        }))
        .and_then(|a| a["browser_download_url"].as_str())
        .unwrap_or("")
        .to_string();

    let unstable_warning = release["prerelease"].as_bool().unwrap_or(false) || version_type(ver) != "release";

    Ok(Some(ReleaseInfo {
        version:  ver.to_string(),
        tag_name: tag,
        body:     release["body"].as_str().unwrap_or("").to_string(),
        html_url: release["html_url"].as_str().unwrap_or("").to_string(),
        asset_url,
        unstable_warning,
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

// ── Console window support ────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ConsoleInfo {
    instance_name: String,
    log_path: String,
}

struct ConsoleState {
    info: std::sync::Mutex<Option<ConsoleInfo>>,
}

#[tauri::command]
fn get_window_type(window: tauri::WebviewWindow) -> String {
    if window.label() == "console" { "console".to_string() } else { "main".to_string() }
}

#[tauri::command]
async fn open_console_window(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let log_path = launcher::mlbv_base()
        .join("instances")
        .join(&instance_name)
        .join("logs")
        .join("latest.log")
        .to_string_lossy()
        .into_owned();

    *app.state::<ConsoleState>().info.lock().unwrap() = Some(ConsoleInfo {
        instance_name: instance_name.clone(),
        log_path,
    });

    // Close existing console window if open
    if let Some(win) = app.get_webview_window("console") {
        let _ = win.close();
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
    }

    tauri::WebviewWindowBuilder::new(
        &app, "console",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title(format!("MLBV Console — {instance_name}"))
    .inner_size(820.0, 580.0)
    .min_inner_size(600.0, 400.0)
    .decorations(false)
    .center()
    .build()
    .map_err(|e| format!("Console window: {e}"))?;

    Ok(())
}

#[tauri::command]
fn get_console_info(app: tauri::AppHandle) -> Option<ConsoleInfo> {
    app.state::<ConsoleState>().info.lock().unwrap().clone()
}

// ── JVM output streaming ────────────────────────────────────────────────────

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

// ── LB config install ────────────────────────────────────────────────────────

#[tauri::command]
async fn install_lb_config(json_url: String, instance_name: String, file_name: String) -> Result<(), String> {
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
struct LbInstallTarget {
    name: String,
}

#[tauri::command]
fn get_lb_installable_instances() -> Vec<LbInstallTarget> {
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
struct FoundInstance {
    name: String,
    instance_type: String,
    mc_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
    build_id: Option<u32>,
}

#[tauri::command]
fn scan_instances() -> Vec<FoundInstance> {
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
fn save_instance_metadata(
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
fn rename_instance_data(old_name: String, new_name: String) -> Result<(), String> {
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

#[derive(serde::Serialize)]
struct ModInfo {
    filename: String,
}

#[tauri::command]
fn list_mods(instance_name: String) -> Vec<ModInfo> {
    if launcher::valid_instance_name(&instance_name).is_err() { return vec![]; }
    let mods_dir = launcher::instances_dir().join(&instance_name).join("mods");
    if !mods_dir.exists() { return vec![]; }
    std::fs::read_dir(&mods_dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.ends_with(".jar") { Some(ModInfo { filename: name }) }
                    else { None }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
fn delete_mods(instance_name: String, filenames: Vec<String>) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let mods_dir = launcher::instances_dir().join(&instance_name).join("mods");
    for filename in &filenames {
        if filename.contains('/') || filename.contains('\\') { continue; }
        let path = mods_dir.join(filename);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("Cannot delete {filename}: {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
fn add_mod_file(instance_name: String, filename: String, data: Vec<u8>) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let safe: String = filename.chars().filter(|&c| c != '/' && c != '\\' && c != '\0').collect();
    if !safe.ends_with(".jar") { return Err("Only .jar files are supported".to_string()); }
    let mods_dir = launcher::instances_dir().join(&instance_name).join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    std::fs::write(mods_dir.join(&safe), &data).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_mods_folder(app: tauri::AppHandle, instance_name: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let mods_dir = launcher::instances_dir().join(&instance_name).join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    app.opener()
       .open_path(mods_dir.to_string_lossy().as_ref(), None::<&str>)
       .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_instance_data(instance_name: String) -> Result<(), String> {
    launcher::valid_instance_name(&instance_name).map_err(|e| e.to_string())?;
    let inst_dir = launcher::instances_dir().join(&instance_name);
    if inst_dir.exists() {
        std::fs::remove_dir_all(&inst_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(serde::Serialize, Clone)]
struct LoaderVersionInfo {
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

#[tauri::command]
async fn get_loader_versions(mc_ver: String, loader: String) -> Result<Vec<LoaderVersionInfo>, String> {
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
            let items: Vec<LoaderVersionInfo> = xml.lines()
                .filter_map(|l| {
                    let l = l.trim();
                    if l.starts_with("<version>") && l.ends_with("</version>") {
                        Some(l[9..l.len()-10].to_string())
                    } else { None }
                })
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
            let items: Vec<LoaderVersionInfo> = xml.lines()
                .filter_map(|l| {
                    let l = l.trim();
                    if l.starts_with("<version>") && l.ends_with("</version>") {
                        Some(l[9..l.len()-10].to_string())
                    } else { None }
                })
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(launcher::GameState::new())
        .manage(ConsoleState { info: std::sync::Mutex::new(None) })
        .invoke_handler(tauri::generate_handler![
            get_game_dir,
            launch_game,
            get_lb_branches,
            get_lb_versions,
            microsoft_login,
            refresh_ms_token,
            rename_instance_data,
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
            get_window_type,
            open_game_dir,
            open_console_window,
            get_console_info,
            poll_jvm_output,
            install_lb_config,
            get_lb_installable_instances,
            scan_instances,
            save_instance_metadata,
            list_mods,
            delete_mods,
            add_mod_file,
            open_mods_folder,
            delete_instance_data,
            get_loader_versions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

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
}
