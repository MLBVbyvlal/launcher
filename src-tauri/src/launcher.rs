use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use tauri::{Emitter, Manager};
use tokio::sync::Semaphore;

// ─── Game process state (shared across launch / stop commands) ───────────────

pub struct GameState {
    pub child:     Mutex<Option<std::process::Child>>,
    pub cancel_dl: Arc<AtomicBool>,
    pub pause_dl:  Arc<AtomicBool>,
    pub dl_bytes:  Arc<AtomicU64>,
    pub jvm_lines: Arc<Mutex<Vec<String>>>,
}
impl GameState {
    pub fn new() -> Self {
        GameState {
            child:     Mutex::new(None),
            cancel_dl: Arc::new(AtomicBool::new(false)),
            pause_dl:  Arc::new(AtomicBool::new(false)),
            dl_bytes:  Arc::new(AtomicU64::new(0)),
            jvm_lines: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

// RAII guard that aborts a tokio task on drop
struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) { self.0.abort(); }
}

// ─── Mojang API types ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<ManifestEntry>,
}

#[derive(Deserialize)]
struct ManifestEntry {
    id: String,
    url: String,
}

#[derive(Deserialize)]
struct VersionJson {
    id: String,
    #[serde(rename = "mainClass")]
    main_class: String,
    // Pre-1.13 style
    #[serde(rename = "minecraftArguments", default)]
    minecraft_arguments: Option<String>,
    // 1.13+ style
    #[serde(default)]
    arguments: Option<NewArguments>,
    #[serde(rename = "assetIndex")]
    asset_index: AssetIndexRef,
    downloads: ClientDownloads,
    libraries: Vec<Library>,
    #[serde(rename = "javaVersion", default)]
    java_version: Option<JavaVersionReq>,
}

#[derive(Deserialize)]
struct NewArguments {
    #[serde(default)]
    game: Vec<Arg>,
    #[serde(default)]
    jvm: Vec<Arg>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Arg {
    Plain(String),
    Conditional { rules: Vec<ArgRule>, value: ArgValue },
}

#[derive(Deserialize)]
struct ArgRule {
    action: String,
    #[serde(default)]
    os: Option<OsCondition>,
    #[serde(default)]
    features: Option<HashMap<String, bool>>,
}

#[derive(Deserialize)]
struct OsCondition {
    name: Option<String>,
    #[serde(default)]
    arch: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
struct AssetIndexRef {
    id: String,
    url: String,
}

#[derive(Deserialize)]
struct ClientDownloads {
    client: FileRef,
}

#[derive(Deserialize)]
struct FileRef {
    url: String,
    sha1: String,
    size: u64,
}

#[derive(Deserialize)]
struct Library {
    name: String,
    #[serde(default)]
    downloads: Option<LibDownloads>,
    #[serde(default)]
    rules: Vec<ArgRule>,
    #[serde(default)]
    natives: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct LibDownloads {
    #[serde(default)]
    artifact: Option<Artifact>,
    #[serde(default)]
    classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize)]
struct Artifact {
    path: String,
    url: String,
    sha1: String,
    size: u64,
}

#[derive(Deserialize)]
pub struct JavaVersionReq {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObj>,
}

#[derive(Deserialize)]
struct AssetObj {
    hash: String,
    size: u64,
}

// ─── Progress event ───────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct LaunchProgress {
    pub stage: String,
    pub progress: f32,
    pub message: String,
}

fn progress(app: &tauri::AppHandle, stage: &str, pct: f32, msg: &str) {
    let _ = app.emit("launch-progress", LaunchProgress {
        stage: stage.into(),
        progress: pct,
        message: msg.into(),
    });
}

// ─── Instance dir ─────────────────────────────────────────────────────────────

pub fn instances_dir() -> PathBuf {
    mlbv_base().join("instances")
}

pub fn shared_data_dir() -> PathBuf {
    mlbv_base().join("shared")
}

pub fn mlbv_base() -> PathBuf {
    if cfg!(windows) {
        std::env::var("APPDATA").map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".")).join("mlbv")
    } else {
        std::env::var("HOME").map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".")).join(".mlbv")
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

async fn download_assets_parallel(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    objects: &HashMap<String, AssetObj>,
    objs_dir: &PathBuf,
    concurrent: u32,
    pct_start: f32,
    pct_range: f32,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    bytes_dl: Arc<AtomicU64>,
) {
    let total = objects.len();
    if total == 0 { return; }
    let sem  = Arc::new(Semaphore::new(concurrent.max(1) as usize));
    let done = Arc::new(AtomicUsize::new(0));
    let mut set = tokio::task::JoinSet::<()>::new();
    for (_, obj) in objects {
        if cancel.load(Ordering::Relaxed) { break; }
        let prefix   = obj.hash[..2].to_string();
        let hash     = obj.hash.clone();
        let size     = obj.size;
        let obj_dir  = objs_dir.join(&prefix);
        let obj_path = obj_dir.join(&hash);
        let sem_c    = sem.clone();
        let done_c   = done.clone();
        let client_c = client.clone();
        let app_c    = app.clone();
        let cancel_c = cancel.clone();
        let pause_c  = pause.clone();
        let bytes_c  = bytes_dl.clone();
        set.spawn(async move {
            let _permit = sem_c.acquire_owned().await.unwrap();
            if cancel_c.load(Ordering::Relaxed) { return; }
            // Pause loop
            while pause_c.load(Ordering::Relaxed) {
                if cancel_c.load(Ordering::Relaxed) { return; }
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            }
            if !is_valid_file(&obj_path, size) {
                let _ = fs::create_dir_all(&obj_dir);
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    prefix, hash
                );
                if let Ok(resp) = client_c.get(&url).send().await {
                    if let Ok(body) = resp.bytes().await {
                        bytes_c.fetch_add(body.len() as u64, Ordering::Relaxed);
                        let _ = fs::write(&obj_path, body);
                    }
                }
            }
            let n = done_c.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 50 == 0 || n == total {
                let pct = pct_start + (n as f32 / total as f32) * pct_range;
                progress(&app_c, "download", pct, &format!("Assets ({n}/{total})…"));
            }
        });
    }
    while set.join_next().await.is_some() {}
}

pub async fn run(
    app: tauri::AppHandle,
    version_id: String,
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<()> {
    let shared_dir = shared_data_dir();
    let game_dir = instances_dir().join(&instance_name);
    for d in &["saves","screenshots","resourcepacks","texturepacks","shaderpacks",
               "logs","crash-reports","config","datapacks","mods"] {
        let _ = fs::create_dir_all(game_dir.join(d));
    }
    fs::create_dir_all(&shared_dir)?;
    fs::create_dir_all(&game_dir)?;
    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()?;

    // Grab cancel/pause/bytes flags from shared state
    let state = app.state::<GameState>();
    let cancel_dl = state.cancel_dl.clone();
    let pause_dl  = state.pause_dl.clone();
    let dl_bytes  = state.dl_bytes.clone();
    cancel_dl.store(false, Ordering::Relaxed);
    pause_dl.store(false, Ordering::Relaxed);
    dl_bytes.store(0, Ordering::Relaxed);

    // Speed monitor: emits "download-speed" every second while running
    let _speed_guard = {
        let b = dl_bytes.clone();
        let a = app.clone();
        let h = tokio::spawn(async move {
            let mut last = 0u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let cur = b.load(Ordering::Relaxed);
                let bps = cur.saturating_sub(last);
                last = cur;
                let _ = a.emit("download-speed", serde_json::json!({ "bps": bps }));
            }
        });
        AbortOnDrop(h)
    };

    // 1. Fetch manifest
    progress(&app, "fetch", 3.0, "Fetching version manifest…");
    let manifest: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await
        .context("Cannot reach Mojang servers")?
        .json().await?;

    let entry = manifest.versions.iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| anyhow!("Version '{}' not found in manifest", version_id))?;

    // 2. Fetch version JSON
    progress(&app, "fetch", 8.0, "Fetching version info…");
    let ver_text = client.get(&entry.url).send().await?.text().await?;
    let ver: VersionJson = serde_json::from_str(&ver_text)?;

    // Version dirs: shared (cache/backup) + per-instance copy
    let shared_ver_dir = shared_dir.join("versions").join(&version_id);
    let inst_ver_dir   = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&shared_ver_dir)?;
    fs::create_dir_all(&inst_ver_dir)?;
    if !shared_ver_dir.join(format!("{version_id}.json")).exists() {
        fs::write(shared_ver_dir.join(format!("{version_id}.json")), &ver_text)?;
    }
    if !inst_ver_dir.join(format!("{version_id}.json")).exists() {
        fs::write(inst_ver_dir.join(format!("{version_id}.json")), &ver_text)?;
    }

    // Client JAR — instance primary, shared is backup/cache for instant re-use
    let jar_shared = shared_ver_dir.join(format!("{version_id}.jar"));
    let jar_path   = inst_ver_dir.join(format!("{version_id}.jar"));
    if !is_valid_file(&jar_path, ver.downloads.client.size) {
        if is_valid_file(&jar_shared, ver.downloads.client.size) {
            fs::copy(&jar_shared, &jar_path).context("Copy JAR from shared")?;
        } else {
            progress(&app, "download", 12.0, "Downloading Minecraft client…");
            download_file(&client, &ver.downloads.client.url, &jar_shared).await
                .context("Failed to download Minecraft client")?;
            fs::copy(&jar_shared, &jar_path).context("Copy JAR to instance")?;
        }
    } else if !is_valid_file(&jar_shared, ver.downloads.client.size) {
        let _ = fs::copy(&jar_path, &jar_shared);
    }

    // 4. Download libraries
    let libs_dir = shared_dir.join("libraries");
    let natives_dir = inst_ver_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;

    let mut classpath: Vec<String> = Vec::new();
    let total_libs = ver.libraries.len();

    for (i, lib) in ver.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        if !lib_allowed(lib) { continue; }

        // Pause loop
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }

        let pct = 15.0 + (i as f32 / total_libs as f32) * 30.0;
        if i % 8 == 0 {
            progress(&app, "download", pct, &format!("Libraries ({i}/{total_libs})…"));
        }

        let Some(dl) = &lib.downloads else { continue };

        // Main jar
        if let Some(art) = &dl.artifact {
            let path = libs_dir.join(&art.path);
            fs::create_dir_all(path.parent().unwrap())?;
            if !is_valid_file(&path, art.size) {
                let _ = download_file(&client, &art.url, &path).await;
            }
            classpath.push(path.to_string_lossy().into_owned());
        }

        // Native classifier
        if let Some(natives_map) = &lib.natives {
            let key = os_classifier_key();
            if let Some(classifier) = natives_map.get(key) {
                let classifier = classifier.replace("${arch}", arch_bits());
                if let Some(classifiers) = &dl.classifiers {
                    if let Some(nat) = classifiers.get(&classifier) {
                        let path = libs_dir.join(&nat.path);
                        fs::create_dir_all(path.parent().unwrap())?;
                        if !is_valid_file(&path, nat.size) {
                            let _ = download_file(&client, &nat.url, &path).await;
                        }
                        let _ = extract_natives(&path, &natives_dir);
                    }
                }
            }
        }
    }

    // Client JAR goes at the END of classpath
    classpath.push(jar_path.to_string_lossy().into_owned());

    // 5. Download asset index
    progress(&app, "download", 48.0, "Downloading asset index…");
    let idx_dir = shared_dir.join("assets").join("indexes");
    fs::create_dir_all(&idx_dir)?;
    let idx_path = idx_dir.join(format!("{}.json", ver.asset_index.id));
    if !idx_path.exists() {
        let idx_text = client.get(&ver.asset_index.url).send().await?.text().await?;
        fs::write(&idx_path, &idx_text)?;
    }

    // 6. Download assets (parallel)
    let idx_text = fs::read_to_string(&idx_path)?;
    let idx: AssetIndex = serde_json::from_str(&idx_text)?;
    let objs_dir = shared_dir.join("assets").join("objects");
    let total_assets = idx.objects.len();
    progress(&app, "download", 50.0, &format!("Assets (0/{total_assets})…"));
    download_assets_parallel(&app, &client, &idx.objects, &objs_dir, concurrent_downloads, 50.0, 35.0,
        cancel_dl.clone(), pause_dl.clone(), dl_bytes.clone()).await;
    if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
    progress(&app, "download", 85.0, &format!("Assets ({total_assets}/{total_assets})…"));

    // 7. Find or auto-download Java
    progress(&app, "launch", 88.0, "Finding Java runtime…");
    let java = ensure_java(&app, &client, &shared_dir, ver.java_version.as_ref()).await
        .context("Failed to obtain Java runtime")?;

    // 8. Build and run command
    progress(&app, "launch", 93.0, "Starting Minecraft…");

    let classpath_str = classpath.join(if cfg!(windows) { ";" } else { ":" });

    let token = if offline { "0".to_string() } else { access_token.clone() };
    let user_type = if offline { "offline" } else { "msa" };

    let vars: HashMap<&str, String> = HashMap::from([
        ("${auth_player_name}", username.clone()),
        ("${version_name}", version_id.clone()),
        ("${game_directory}", game_dir.to_string_lossy().into_owned()),      // per-instance
        ("${assets_root}", shared_dir.join("assets").to_string_lossy().into_owned()),
        ("${assets_index_name}", ver.asset_index.id.clone()),
        ("${auth_uuid}", uuid.clone()),
        ("${auth_access_token}", token.clone()),
        ("${user_type}", user_type.to_string()),
        ("${version_type}", "release".to_string()),
        ("${user_properties}", "{}".to_string()),
        ("${natives_directory}", natives_dir.to_string_lossy().into_owned()),
        ("${launcher_name}", "MLBV".to_string()),
        ("${launcher_version}", "1.0".to_string()),
        ("${classpath}", classpath_str.clone()),
        ("${classpath_separator}", if cfg!(windows) { ";".to_string() } else { ":".to_string() }),
        ("${resolution_width}", "854".to_string()),
        ("${resolution_height}", "480".to_string()),
    ]);

    let replace = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars { out = out.replace(k, v); }
        out
    };

    let mut cmd_args: Vec<String> = Vec::new();

    if let Some(new_args) = &ver.arguments {
        // 1.13+ format: use JVM args from JSON
        for arg in &new_args.jvm {
            resolve_arg(arg, &replace, &mut cmd_args);
        }
        cmd_args.push(format!("-Xmx{}m", max_ram_mb));
        cmd_args.push("-Xms256m".to_string());
        cmd_args.push(ver.main_class.clone());
        for arg in &new_args.game {
            resolve_arg(arg, &replace, &mut cmd_args);
        }
    } else {
        // Pre-1.13 format: manual JVM args
        cmd_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        cmd_args.push(format!("-Dminecraft.launcher.brand=MLBV"));
        cmd_args.push(format!("-Dminecraft.launcher.version=1.0"));
        cmd_args.push(format!("-Xmx{}m", max_ram_mb));
        cmd_args.push("-Xms256m".to_string());
        cmd_args.push("-cp".to_string());
        cmd_args.push(classpath_str);
        cmd_args.push(ver.main_class.clone());
        if let Some(old) = &ver.minecraft_arguments {
            for part in old.split_whitespace() {
                cmd_args.push(replace(part));
            }
        }
    }

    let jvm_lines = app.state::<GameState>().jvm_lines.clone();
    jvm_lines.lock().unwrap().clear();
    let mut java_cmd = std::process::Command::new(&java);
    java_cmd.args(&cmd_args).current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = java_cmd
        .spawn()
        .with_context(|| format!("Failed to start Java from {:?}", java))?;
    if let Some(out) = child.stdout.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(out).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }
    if let Some(err) = child.stderr.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(err).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }

    progress(&app, "launch", 100.0, "Minecraft launched!");

    *app.state::<GameState>().child.lock().unwrap() = Some(child);
    let _ = app.emit("game-running", true);

    let app_mon = app.clone();
    let game_dir_mon = game_dir.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let state = app_mon.state::<GameState>();
            let mut guard = state.child.lock().unwrap();
            match guard.as_mut() {
                Some(c) => { if let Ok(Some(status)) = c.try_wait() {
                    *guard = None; drop(guard);
                    if !status.success() {
                        let log_path = game_dir_mon.join("logs").join("latest.log");
                        let log_tail = {
                            let gs = app_mon.state::<GameState>();
                            let captured = gs.jvm_lines.lock().unwrap();
                            if !captured.is_empty() {
                                let total = captured.len();
                                captured[total.saturating_sub(80)..].join("\n")
                            } else {
                                std::fs::read_to_string(&log_path)
                                    .map(|s| { let v: Vec<&str> = s.lines().collect(); v[v.len().saturating_sub(80)..].join("\n") })
                                    .unwrap_or_else(|_| "No output captured.".into())
                            }
                        };
                        let _ = app_mon.emit("game-crashed", serde_json::json!({
                            "exitCode": status.code().unwrap_or(-1),
                            "log": log_tail,
                            "logPath": log_path.to_string_lossy().into_owned(),
                        }));
                    }
                    let _ = app_mon.emit("game-running", false);
                    break;
                }}
                None => break,
            }
        }
    });

    Ok(())
}

// ─── LiquidBounce launcher ───────────────────────────────────────────────────

#[derive(Deserialize)]
struct LbManifest {
    build: LbManifestBuild,
    #[serde(default)]
    mods: Vec<LbMod>,
    #[serde(default)]
    repositories: HashMap<String, String>,
}

#[derive(Deserialize)]
struct LbManifestBuild {
    mc_version: String,
    lb_version: String,
    fabric_loader_version: String,
}

#[derive(Deserialize)]
struct LbMod {
    required: bool,
    name: String,
    source: serde_json::Value,
}

#[derive(Deserialize)]
struct FabricProfile {
    #[serde(rename = "mainClass")]
    main_class: String,
    #[serde(default)]
    libraries: Vec<FabricLibrary>,
    #[serde(default)]
    arguments: Option<FabricArguments>,
}

#[derive(Deserialize)]
struct FabricLibrary {
    name: String,
    url: String,
}

#[derive(Deserialize)]
struct FabricArguments {
    #[serde(default)]
    jvm: Vec<serde_json::Value>,
}

/// Try to download one Modrinth mod. Silently skips if unavailable for this MC version.
async fn download_modrinth_mod(
    client: &reqwest::Client,
    slug: &str,
    mc_ver: &str,
    mods_dir: &PathBuf,
    loader: &str,
) {
    let url = format!(
        "https://api.modrinth.com/v2/project/{slug}/version?game_versions=[\"{mc_ver}\"]&loaders=[\"{loader}\"]"
    );
    let resp = match client
        .get(&url)
        .header("User-Agent", "MLBV/1.0 (github.com/MLBVbyvlalikoffc/launcher)")
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
                let _ = download_file(client, dl_url, &dest).await;
            }
        }
    }
}

pub async fn run_lb(
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
) -> Result<()> {
    let shared_dir = shared_data_dir();
    let game_dir = instances_dir().join(&instance_name);
    for d in &["saves","screenshots","resourcepacks","texturepacks","shaderpacks",
               "logs","crash-reports","config","datapacks","mods"] {
        let _ = fs::create_dir_all(game_dir.join(d));
    }
    let mods_dir = game_dir.join("mods");
    fs::create_dir_all(&shared_dir)?;
    fs::create_dir_all(&game_dir)?;
    fs::create_dir_all(&mods_dir)?;

    let client = reqwest::Client::builder().user_agent("MLBV/1.0").build()?;

    let state = app.state::<GameState>();
    let cancel_dl = state.cancel_dl.clone();
    let pause_dl  = state.pause_dl.clone();
    let dl_bytes  = state.dl_bytes.clone();
    cancel_dl.store(false, Ordering::Relaxed);
    pause_dl.store(false, Ordering::Relaxed);
    dl_bytes.store(0, Ordering::Relaxed);

    let _speed_guard = {
        let b = dl_bytes.clone();
        let a = app.clone();
        let h = tokio::spawn(async move {
            let mut last = 0u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let cur = b.load(Ordering::Relaxed);
                let bps = cur.saturating_sub(last);
                last = cur;
                let _ = a.emit("download-speed", serde_json::json!({ "bps": bps }));
            }
        });
        AbortOnDrop(h)
    };

    // 1. Fetch LB launch manifest
    progress(&app, "fetch", 5.0, "Fetching LiquidBounce manifest…");
    let manifest_text = client
        .get(format!("https://api.liquidbounce.net/api/v1/version/launch/{}", build_id))
        .send().await.context("Cannot reach LiquidBounce API")?
        .text().await?;
    let manifest: LbManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| anyhow!("Manifest parse: {e}"))?;

    let loader_ver = &manifest.build.fabric_loader_version;

    // 2. Download vanilla MC first (shared libraries/assets)
    progress(&app, "fetch", 10.0, "Setting up vanilla Minecraft…");
    let mc_ver = mc_version.clone();
    // Re-use vanilla download logic inline
    let mf: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await?.json().await?;
    let entry = mf.versions.iter().find(|v| v.id == mc_ver)
        .ok_or_else(|| anyhow!("MC version {} not found", mc_ver))?;
    let ver_text = client.get(&entry.url).send().await?.text().await?;
    let ver: VersionJson = serde_json::from_str(&ver_text)?;

    let shared_ver_dir = shared_dir.join("versions").join(&mc_ver);
    let inst_ver_dir   = game_dir.join("versions").join(&mc_ver);
    fs::create_dir_all(&shared_ver_dir)?;
    fs::create_dir_all(&inst_ver_dir)?;
    if !shared_ver_dir.join(format!("{mc_ver}.json")).exists() { fs::write(shared_ver_dir.join(format!("{mc_ver}.json")), &ver_text)?; }
    if !inst_ver_dir.join(format!("{mc_ver}.json")).exists() { fs::write(inst_ver_dir.join(format!("{mc_ver}.json")), &ver_text)?; }

    let jar_shared = shared_ver_dir.join(format!("{mc_ver}.jar"));
    let jar_path   = inst_ver_dir.join(format!("{mc_ver}.jar"));
    if !is_valid_file(&jar_path, ver.downloads.client.size) {
        if is_valid_file(&jar_shared, ver.downloads.client.size) {
            fs::copy(&jar_shared, &jar_path).context("Copy JAR from shared")?;
        } else {
            progress(&app, "download", 14.0, "Downloading Minecraft client…");
            download_file(&client, &ver.downloads.client.url, &jar_shared).await?;
            fs::copy(&jar_shared, &jar_path).context("Copy JAR to instance")?;
        }
    } else if !is_valid_file(&jar_shared, ver.downloads.client.size) {
        let _ = fs::copy(&jar_path, &jar_shared);
    }

    // 3. Download vanilla libraries
    let libs_dir = shared_dir.join("libraries");
    let natives_dir = inst_ver_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;
    let mut classpath: Vec<String> = Vec::new();
    let total = ver.libraries.len();
    for (i, lib) in ver.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        if !lib_allowed(lib) { continue; }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 10 == 0 {
            let pct = 16.0 + (i as f32 / total as f32) * 22.0;
            progress(&app, "download", pct, &format!("Vanilla libraries ({i}/{total})…"));
        }
        let Some(dl) = &lib.downloads else { continue };
        if let Some(art) = &dl.artifact {
            let p = libs_dir.join(&art.path);
            fs::create_dir_all(p.parent().unwrap())?;
            if !is_valid_file(&p, art.size) { let _ = download_file(&client, &art.url, &p).await; }
            classpath.push(p.to_string_lossy().into_owned());
        }
        if let Some(natives_map) = &lib.natives {
            let key = os_classifier_key();
            if let Some(classifier) = natives_map.get(key) {
                let classifier = classifier.replace("${arch}", arch_bits());
                if let Some(classifiers) = &dl.classifiers {
                    if let Some(nat) = classifiers.get(&classifier) {
                        let p = libs_dir.join(&nat.path);
                        fs::create_dir_all(p.parent().unwrap())?;
                        if !is_valid_file(&p, nat.size) { let _ = download_file(&client, &nat.url, &p).await; }
                        let _ = extract_natives(&p, &natives_dir);
                    }
                }
            }
        }
    }

    // 4. Download assets (parallel)
    progress(&app, "download", 40.0, "Downloading asset index…");
    let idx_dir = shared_dir.join("assets").join("indexes");
    fs::create_dir_all(&idx_dir)?;
    let idx_path = idx_dir.join(format!("{}.json", ver.asset_index.id));
    if !idx_path.exists() {
        let t = client.get(&ver.asset_index.url).send().await?.text().await?;
        fs::write(&idx_path, &t)?;
    }
    let idx: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;
    let objs_dir = shared_dir.join("assets").join("objects");
    let total_a = idx.objects.len();
    progress(&app, "download", 42.0, &format!("Assets (0/{total_a})…"));
    download_assets_parallel(&app, &client, &idx.objects, &objs_dir, concurrent_downloads, 42.0, 20.0,
        cancel_dl.clone(), pause_dl.clone(), dl_bytes.clone()).await;
    if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
    progress(&app, "download", 62.0, &format!("Assets ({total_a}/{total_a})…"));

    // 5. Fetch Fabric profile
    progress(&app, "download", 63.0, "Fetching Fabric Loader profile…");
    let fabric_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        mc_ver, loader_ver
    );
    let fabric_text = client.get(&fabric_url).send().await?.text().await?;
    let fabric: FabricProfile = serde_json::from_str(&fabric_text)
        .map_err(|e| anyhow!("Fabric profile parse: {e}"))?;

    // 6. Download Fabric libraries
    let fabric_libs_dir = shared_dir.join("libraries");
    let total_fl = fabric.libraries.len();
    for (i, flib) in fabric.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 5 == 0 {
            let pct = 65.0 + (i as f32 / total_fl.max(1) as f32) * 10.0;
            progress(&app, "download", pct, &format!("Fabric libraries ({i}/{total_fl})…"));
        }
        // Parse Maven coords: group:artifact:version
        let parts: Vec<&str> = flib.name.splitn(3, ':').collect();
        if parts.len() < 3 { continue; }
        let (group, artifact, version) = (parts[0], parts[1], parts[2]);
        let group_path = group.replace('.', "/");
        let jar_name = format!("{artifact}-{version}.jar");
        let rel_path = format!("{group_path}/{artifact}/{version}/{jar_name}");
        let dest = fabric_libs_dir.join(&rel_path);
        if !dest.exists() {
            fs::create_dir_all(dest.parent().unwrap())?;
            let url = format!("{}{}", flib.url.trim_end_matches('/'), format!("/{rel_path}"));
            let _ = download_file(&client, &url, &dest).await;
        }
        classpath.push(dest.to_string_lossy().into_owned());
    }

    // 7. Download LB mods + required mods into instance mods dir
    let total_mods = manifest.mods.len();
    for (i, m) in manifest.mods.iter().enumerate() {
        if !m.required { continue; }
        let pct = 76.0 + (i as f32 / total_mods.max(1) as f32) * 14.0;
        progress(&app, "download", pct, &format!("Скачиваю: {}…", m.name));
        let src_type = m.source.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match src_type {
            "skip" => {
                // New API: source has "url" + "artifactName"
                // Old API: source has "skip_pid" or "pid"
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
                        progress(&app, "download", pct,
                            &format!("Открываем страницу скачивания {}…  нажмите «Download»", m.name));
                        download_lb_mod_webview(&app, &dl_url, &dest).await
                            .with_context(|| format!("Не удалось скачать мод {}", m.name))?;
                    }
                }
            }
            "repository" => {
                let repo     = m.source.get("repository").and_then(|v| v.as_str()).unwrap_or("");
                let artifact = m.source.get("artifact").and_then(|v| v.as_str()).unwrap_or("");
                // Base URL: from manifest repositories map, or well-known fallbacks
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
                    let group_path = group.replace('.', "/");
                    let jar_name = format!("{art}-{ver}.jar");
                    let dest = mods_dir.join(&jar_name);
                    if !is_valid_file(&dest, 0) {
                        let url = format!("{}/{group_path}/{art}/{ver}/{jar_name}",
                            base.trim_end_matches('/'));
                        let _ = download_file(&client, &url, &dest).await;
                    }
                }
            }
            _ => {}
        }
    }

    // 7b. Extra mods from Modrinth (version-aware, skip if unavailable)
    let lb_extra_mods = &[
        "sodium", "modmenu", "immediatelyfast", "iris", "lithium",
        "viafabricplus", "exploitfixer",
    ];
    for slug in lb_extra_mods {
        let pct_extra = 90.0 + (lb_extra_mods.iter().position(|s| s == slug).unwrap_or(0) as f32 / lb_extra_mods.len() as f32) * 4.0;
        progress(&app, "download", pct_extra, &format!("Extra mods: {}…", slug));
        download_modrinth_mod(&client, slug, &mc_ver, &mods_dir, "fabric").await;
    }

    // Client JAR at end of classpath
    classpath.push(jar_path.to_string_lossy().into_owned());

    // 8. Find or auto-download Java
    progress(&app, "launch", 92.0, "Finding Java runtime…");
    let java = ensure_java(&app, &client, &shared_dir, ver.java_version.as_ref()).await
        .context("Failed to obtain Java runtime")?;

    // 9. Build command with Fabric main class
    progress(&app, "launch", 95.0, "Starting LiquidBounce…");
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath_str = classpath.join(sep);
    let token = if offline { "0".to_string() } else { access_token };
    let user_type = if offline { "offline" } else { "msa" };

    let vars: HashMap<&str, String> = HashMap::from([
        ("${auth_player_name}", username.clone()),
        ("${version_name}", format!("fabric-loader-{loader_ver}-{mc_ver}")),
        ("${game_directory}", game_dir.to_string_lossy().into_owned()),
        ("${assets_root}", shared_dir.join("assets").to_string_lossy().into_owned()),
        ("${assets_index_name}", ver.asset_index.id.clone()),
        ("${auth_uuid}", uuid.clone()),
        ("${auth_access_token}", token),
        ("${user_type}", user_type.to_string()),
        ("${version_type}", "release".to_string()),
        ("${user_properties}", "{}".to_string()),
        ("${natives_directory}", natives_dir.to_string_lossy().into_owned()),
        ("${launcher_name}", "MLBV".to_string()),
        ("${launcher_version}", "1.0".to_string()),
        ("${classpath}", classpath_str.clone()),
        ("${classpath_separator}", sep.to_string()),
        ("${resolution_width}", "854".to_string()),
        ("${resolution_height}", "480".to_string()),
    ]);
    let replace = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars { out = out.replace(k, v); }
        out
    };

    let mut cmd_args: Vec<String> = Vec::new();
    // Fabric JVM args
    if let Some(fa) = &fabric.arguments {
        for v in &fa.jvm {
            if let Some(s) = v.as_str() { cmd_args.push(replace(s)); }
        }
    }
    // Standard JVM args from vanilla profile
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.jvm { resolve_arg(arg, &replace, &mut cmd_args); }
    } else {
        cmd_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        cmd_args.push("-Dminecraft.launcher.brand=MLBV".to_string());
        cmd_args.push("-cp".to_string());
        cmd_args.push(classpath_str);
    }
    cmd_args.push(format!("-Xmx{}m", max_ram_mb));
    cmd_args.push("-Xms256m".to_string());
    cmd_args.push(fabric.main_class.clone());
    // Game args
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.game { resolve_arg(arg, &replace, &mut cmd_args); }
    } else if let Some(old) = &ver.minecraft_arguments {
        for part in old.split_whitespace() { cmd_args.push(replace(part)); }
    }
    // Add mods dir via Fabric's fabric.addMods JVM property (for loading from mods/)
    // Fabric automatically scans game_dir/mods, so no extra arg needed

    let jvm_lines = app.state::<GameState>().jvm_lines.clone();
    jvm_lines.lock().unwrap().clear();
    let mut java_cmd = std::process::Command::new(&java);
    java_cmd.args(&cmd_args).current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = java_cmd
        .spawn()
        .with_context(|| format!("Failed to start Java from {:?}", java))?;
    if let Some(out) = child.stdout.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(out).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }
    if let Some(err) = child.stderr.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(err).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }

    progress(&app, "launch", 100.0, "LiquidBounce launched!");

    *app.state::<GameState>().child.lock().unwrap() = Some(child);
    let _ = app.emit("game-running", true);

    let app_mon = app.clone();
    let game_dir_mon = game_dir.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let state = app_mon.state::<GameState>();
            let mut guard = state.child.lock().unwrap();
            match guard.as_mut() {
                Some(c) => { if let Ok(Some(status)) = c.try_wait() {
                    *guard = None; drop(guard);
                    if !status.success() {
                        let log_path = game_dir_mon.join("logs").join("latest.log");
                        let log_tail = {
                            let gs = app_mon.state::<GameState>();
                            let captured = gs.jvm_lines.lock().unwrap();
                            if !captured.is_empty() {
                                let total = captured.len();
                                captured[total.saturating_sub(80)..].join("\n")
                            } else {
                                std::fs::read_to_string(&log_path)
                                    .map(|s| { let v: Vec<&str> = s.lines().collect(); v[v.len().saturating_sub(80)..].join("\n") })
                                    .unwrap_or_else(|_| "No output captured.".into())
                            }
                        };
                        let _ = app_mon.emit("game-crashed", serde_json::json!({
                            "exitCode": status.code().unwrap_or(-1),
                            "log": log_tail,
                            "logPath": log_path.to_string_lossy().into_owned(),
                        }));
                    }
                    let _ = app_mon.emit("game-running", false);
                    break;
                }}
                None => break,
            }
        }
    });

    Ok(())
}

// ─── Fabric (vanilla + Fabric Loader + Fabric API from Modrinth) ─────────────

pub async fn run_fabric(
    app: tauri::AppHandle,
    mc_version: String,
    loader_version: String,
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<()> {
    let shared_dir = shared_data_dir();
    let game_dir   = instances_dir().join(&instance_name);
    let mods_dir   = game_dir.join("mods");
    fs::create_dir_all(&shared_dir)?;
    fs::create_dir_all(&game_dir)?;
    fs::create_dir_all(&mods_dir)?;

    let client = reqwest::Client::builder().user_agent("MLBV/1.0").build()?;

    let state = app.state::<GameState>();
    let cancel_dl = state.cancel_dl.clone();
    let pause_dl  = state.pause_dl.clone();
    let dl_bytes  = state.dl_bytes.clone();
    cancel_dl.store(false, Ordering::Relaxed);
    pause_dl.store(false, Ordering::Relaxed);
    dl_bytes.store(0, Ordering::Relaxed);

    let _speed_guard = {
        let b = dl_bytes.clone();
        let a = app.clone();
        let h = tokio::spawn(async move {
            let mut last = 0u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let cur = b.load(Ordering::Relaxed);
                let bps = cur.saturating_sub(last);
                last = cur;
                let _ = a.emit("download-speed", serde_json::json!({ "bps": bps }));
            }
        });
        AbortOnDrop(h)
    };

    // 1. Resolve Fabric Loader version (use provided, or auto-pick latest stable)
    let loader_ver = if loader_version.is_empty() {
        progress(&app, "fetch", 5.0, "Fetching Fabric Loader version…");
        let loader_list: serde_json::Value = client
            .get(format!("https://meta.fabricmc.net/v2/versions/loader/{}", mc_version))
            .send().await.context("Cannot reach FabricMC API")?
            .json().await?;
        loader_list.as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|e| e["loader"]["stable"].as_bool().unwrap_or(false))
                    .or_else(|| arr.first())
            })
            .and_then(|e| e["loader"]["version"].as_str())
            .ok_or_else(|| anyhow!("No Fabric Loader found for MC {}", mc_version))?
            .to_string()
    } else {
        progress(&app, "fetch", 5.0, &format!("Using Fabric Loader {}…", loader_version));
        loader_version.clone()
    };

    // 2. Download vanilla MC
    progress(&app, "fetch", 10.0, "Setting up vanilla Minecraft…");
    let mc_ver = mc_version.clone();
    let mf: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await?.json().await?;
    let entry = mf.versions.iter().find(|v| v.id == mc_ver)
        .ok_or_else(|| anyhow!("MC version {} not found", mc_ver))?;
    let ver_text = client.get(&entry.url).send().await?.text().await?;
    let ver: VersionJson = serde_json::from_str(&ver_text)?;

    let ver_dir = shared_dir.join("versions").join(&mc_ver);
    fs::create_dir_all(&ver_dir)?;
    let ver_json_path = ver_dir.join(format!("{mc_ver}.json"));
    if !ver_json_path.exists() { fs::write(&ver_json_path, &ver_text)?; }

    let jar_path = ver_dir.join(format!("{mc_ver}.jar"));
    if !is_valid_file(&jar_path, ver.downloads.client.size) {
        progress(&app, "download", 14.0, "Downloading Minecraft client…");
        download_file(&client, &ver.downloads.client.url, &jar_path).await?;
    }

    // 3. Vanilla libraries
    let libs_dir     = shared_dir.join("libraries");
    let natives_dir  = ver_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;
    let mut classpath: Vec<String> = Vec::new();
    let total = ver.libraries.len();
    for (i, lib) in ver.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 10 == 0 {
            let pct = 16.0 + (i as f32 / total as f32) * 22.0;
            progress(&app, "download", pct, &format!("Vanilla libraries ({i}/{total})…"));
        }
        if !lib_allowed(lib) { continue; }
        let Some(dl) = &lib.downloads else { continue };
        if let Some(art) = &dl.artifact {
            let p = libs_dir.join(&art.path);
            fs::create_dir_all(p.parent().unwrap())?;
            if !is_valid_file(&p, art.size) { let _ = download_file(&client, &art.url, &p).await; }
            classpath.push(p.to_string_lossy().into_owned());
        }
        if let Some(natives_map) = &lib.natives {
            let key = os_classifier_key();
            if let Some(classifier) = natives_map.get(key) {
                let classifier = classifier.replace("${arch}", arch_bits());
                if let Some(classifiers) = &dl.classifiers {
                    if let Some(nat) = classifiers.get(&classifier) {
                        let p = libs_dir.join(&nat.path);
                        fs::create_dir_all(p.parent().unwrap())?;
                        if !is_valid_file(&p, nat.size) { let _ = download_file(&client, &nat.url, &p).await; }
                        let _ = extract_natives(&p, &natives_dir);
                    }
                }
            }
        }
    }

    // 4. Assets
    progress(&app, "download", 40.0, "Downloading asset index…");
    let idx_dir = shared_dir.join("assets").join("indexes");
    fs::create_dir_all(&idx_dir)?;
    let idx_path = idx_dir.join(format!("{}.json", ver.asset_index.id));
    if !idx_path.exists() {
        let t = client.get(&ver.asset_index.url).send().await?.text().await?;
        fs::write(&idx_path, &t)?;
    }
    let idx: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;
    let objs_dir  = shared_dir.join("assets").join("objects");
    let total_a   = idx.objects.len();
    progress(&app, "download", 42.0, &format!("Assets (0/{total_a})…"));
    download_assets_parallel(&app, &client, &idx.objects, &objs_dir, concurrent_downloads, 42.0, 20.0,
        cancel_dl.clone(), pause_dl.clone(), dl_bytes.clone()).await;
    if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
    progress(&app, "download", 62.0, &format!("Assets ({total_a}/{total_a})…"));

    // 5. Fabric Loader profile + libraries
    progress(&app, "download", 63.0, "Fetching Fabric Loader profile…");
    let fabric_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        mc_ver, loader_ver
    );
    let fabric_text = client.get(&fabric_url).send().await?.text().await?;
    let fabric: FabricProfile = serde_json::from_str(&fabric_text)
        .map_err(|e| anyhow!("Fabric profile parse: {e}"))?;

    let total_fl = fabric.libraries.len();
    for (i, flib) in fabric.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 5 == 0 {
            let pct = 65.0 + (i as f32 / total_fl.max(1) as f32) * 10.0;
            progress(&app, "download", pct, &format!("Fabric libraries ({i}/{total_fl})…"));
        }
        let parts: Vec<&str> = flib.name.splitn(3, ':').collect();
        if parts.len() < 3 { continue; }
        let (group, artifact, version) = (parts[0], parts[1], parts[2]);
        let group_path = group.replace('.', "/");
        let jar_name   = format!("{artifact}-{version}.jar");
        let rel_path   = format!("{group_path}/{artifact}/{version}/{jar_name}");
        let dest       = libs_dir.join(&rel_path);
        if !dest.exists() {
            fs::create_dir_all(dest.parent().unwrap())?;
            let url = format!("{}/{rel_path}", flib.url.trim_end_matches('/'));
            let _ = download_file(&client, &url, &dest).await;
        }
        classpath.push(dest.to_string_lossy().into_owned());
    }

    // 6. Download Fabric API from Modrinth (project P7dR8mSH)
    progress(&app, "download", 76.0, "Downloading Fabric API from Modrinth…");
    let modrinth_url = format!(
        "https://api.modrinth.com/v2/project/P7dR8mSH/version?game_versions=[\"{mc_ver}\"]&loaders=[\"fabric\"]"
    );
    let versions_resp: serde_json::Value = client
        .get(&modrinth_url)
        .header("User-Agent", "MLBV/1.0 (github.com/MLBVbyvlalikoffc/launcher)")
        .send().await
        .context("Cannot reach Modrinth API")?
        .json().await?;

    if let Some(first_ver) = versions_resp.as_array().and_then(|a| a.first()) {
        if let Some(files) = first_ver["files"].as_array() {
            let jar_file = files.iter().find(|f| {
                f["filename"].as_str().map(|n| n.ends_with(".jar") && !n.contains("-sources")).unwrap_or(false)
            });
            if let Some(f) = jar_file {
                if let (Some(url), Some(name)) = (f["url"].as_str(), f["filename"].as_str()) {
                    let dest = mods_dir.join(name);
                    if !is_valid_file(&dest, 0) {
                        let _ = download_file(&client, url, &dest).await;
                    }
                }
            }
        }
    }

    // MC jar at end of classpath
    classpath.push(jar_path.to_string_lossy().into_owned());

    // 7. Java
    progress(&app, "launch", 92.0, "Finding Java runtime…");
    let java = ensure_java(&app, &client, &shared_dir, ver.java_version.as_ref()).await
        .context("Failed to obtain Java runtime")?;

    // 8. Build command
    progress(&app, "launch", 95.0, "Starting Minecraft (Fabric)…");
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath_str = classpath.join(sep);
    let token     = if offline { "0".to_string() } else { access_token };
    let user_type = if offline { "offline" } else { "msa" };

    let vars: HashMap<&str, String> = HashMap::from([
        ("${auth_player_name}", username.clone()),
        ("${version_name}", format!("fabric-loader-{loader_ver}-{mc_ver}")),
        ("${game_directory}", game_dir.to_string_lossy().into_owned()),
        ("${assets_root}", shared_dir.join("assets").to_string_lossy().into_owned()),
        ("${assets_index_name}", ver.asset_index.id.clone()),
        ("${auth_uuid}", uuid.clone()),
        ("${auth_access_token}", token),
        ("${user_type}", user_type.to_string()),
        ("${version_type}", "release".to_string()),
        ("${user_properties}", "{}".to_string()),
        ("${natives_directory}", natives_dir.to_string_lossy().into_owned()),
        ("${launcher_name}", "MLBV".to_string()),
        ("${launcher_version}", "1.0".to_string()),
        ("${classpath}", classpath_str.clone()),
        ("${classpath_separator}", sep.to_string()),
        ("${resolution_width}", "854".to_string()),
        ("${resolution_height}", "480".to_string()),
    ]);
    let replace = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars { out = out.replace(k, v); }
        out
    };

    let mut cmd_args: Vec<String> = Vec::new();
    if let Some(fa) = &fabric.arguments {
        for v in &fa.jvm { if let Some(s) = v.as_str() { cmd_args.push(replace(s)); } }
    }
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.jvm { resolve_arg(arg, &replace, &mut cmd_args); }
    } else {
        cmd_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        cmd_args.push("-Dminecraft.launcher.brand=MLBV".to_string());
        cmd_args.push("-cp".to_string());
        cmd_args.push(classpath_str);
    }
    cmd_args.push(format!("-Xmx{}m", max_ram_mb));
    cmd_args.push("-Xms256m".to_string());
    cmd_args.push(fabric.main_class.clone());
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.game { resolve_arg(arg, &replace, &mut cmd_args); }
    } else if let Some(old) = &ver.minecraft_arguments {
        for part in old.split_whitespace() { cmd_args.push(replace(part)); }
    }

    let jvm_lines = app.state::<GameState>().jvm_lines.clone();
    jvm_lines.lock().unwrap().clear();
    let mut java_cmd = std::process::Command::new(&java);
    java_cmd.args(&cmd_args).current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = java_cmd
        .spawn()
        .with_context(|| format!("Failed to start Java from {:?}", java))?;
    if let Some(out) = child.stdout.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(out).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }
    if let Some(err) = child.stderr.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(err).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }

    progress(&app, "launch", 100.0, "Minecraft (Fabric) launched!");

    *app.state::<GameState>().child.lock().unwrap() = Some(child);
    let _ = app.emit("game-running", true);

    let app_mon     = app.clone();
    let game_dir_mon = game_dir.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let state = app_mon.state::<GameState>();
            let mut guard = state.child.lock().unwrap();
            match guard.as_mut() {
                Some(c) => { if let Ok(Some(status)) = c.try_wait() {
                    *guard = None; drop(guard);
                    if !status.success() {
                        let log_path = game_dir_mon.join("logs").join("latest.log");
                        let log_tail = {
                            let gs = app_mon.state::<GameState>();
                            let captured = gs.jvm_lines.lock().unwrap();
                            if !captured.is_empty() {
                                let total = captured.len();
                                captured[total.saturating_sub(80)..].join("\n")
                            } else {
                                std::fs::read_to_string(&log_path)
                                    .map(|s| { let v: Vec<&str> = s.lines().collect(); v[v.len().saturating_sub(80)..].join("\n") })
                                    .unwrap_or_else(|_| "No output captured.".into())
                            }
                        };
                        let _ = app_mon.emit("game-crashed", serde_json::json!({
                            "exitCode": status.code().unwrap_or(-1),
                            "log": log_tail,
                            "logPath": log_path.to_string_lossy().into_owned(),
                        }));
                    }
                    let _ = app_mon.emit("game-running", false);
                    break;
                }}
                None => break,
            }
        }
    });

    Ok(())
}

// ─── Parallel URL downloader ─────────────────────────────────────────────────

async fn parallel_download_urls(
    client: &reqwest::Client,
    downloads: &[(String, PathBuf, u64)], // (url, dest_path, expected_size)
    max_concurrent: usize,
) {
    let sem = Arc::new(Semaphore::new(max_concurrent.max(1)));
    let mut set = tokio::task::JoinSet::<()>::new();
    for (url, path, size) in downloads {
        if is_valid_file(path, *size) { continue; }
        let sem_c    = sem.clone();
        let client_c = client.clone();
        let url_c    = url.clone();
        let path_c   = path.clone();
        let size_c   = *size;
        set.spawn(async move {
            let _permit = sem_c.acquire_owned().await.unwrap();
            if !is_valid_file(&path_c, size_c) {
                if let Some(p) = path_c.parent() { let _ = fs::create_dir_all(p); }
                let _ = download_file(&client_c, &url_c, &path_c).await;
            }
        });
    }
    while set.join_next().await.is_some() {}
}

// ─── Quilt launcher ───────────────────────────────────────────────────────────

pub async fn run_quilt(
    app: tauri::AppHandle,
    mc_version: String,
    loader_version: String,
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<()> {
    let shared_dir = shared_data_dir();
    let game_dir   = instances_dir().join(&instance_name);
    let mods_dir   = game_dir.join("mods");
    fs::create_dir_all(&shared_dir)?;
    fs::create_dir_all(&game_dir)?;
    fs::create_dir_all(&mods_dir)?;

    let client = reqwest::Client::builder().user_agent("MLBV/1.0").build()?;

    let state = app.state::<GameState>();
    let cancel_dl = state.cancel_dl.clone();
    let pause_dl  = state.pause_dl.clone();
    let dl_bytes  = state.dl_bytes.clone();
    cancel_dl.store(false, Ordering::Relaxed);
    pause_dl.store(false, Ordering::Relaxed);
    dl_bytes.store(0, Ordering::Relaxed);

    let _speed_guard = {
        let b = dl_bytes.clone();
        let a = app.clone();
        let h = tokio::spawn(async move {
            let mut last = 0u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let cur = b.load(Ordering::Relaxed);
                let bps = cur.saturating_sub(last);
                last = cur;
                let _ = a.emit("download-speed", serde_json::json!({ "bps": bps }));
            }
        });
        AbortOnDrop(h)
    };

    // 1. Resolve Quilt Loader version (use provided, or auto-pick latest)
    let loader_ver = if loader_version.is_empty() {
        progress(&app, "fetch", 5.0, "Fetching Quilt Loader version…");
        let loader_list: serde_json::Value = client
            .get(format!("https://meta.quiltmc.org/v3/versions/loader/{}", mc_version))
            .send().await.context("Cannot reach QuiltMC API")?
            .json().await?;
        loader_list.as_array()
            .and_then(|arr| arr.first())
            .and_then(|e| e["loader"]["version"].as_str())
            .ok_or_else(|| anyhow!("No Quilt Loader found for MC {}", mc_version))?
            .to_string()
    } else {
        progress(&app, "fetch", 5.0, &format!("Using Quilt Loader {}…", loader_version));
        loader_version.clone()
    };

    // 2. Download vanilla MC
    progress(&app, "fetch", 10.0, "Setting up vanilla Minecraft…");
    let mc_ver = mc_version.clone();
    let mf: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await?.json().await?;
    let entry = mf.versions.iter().find(|v| v.id == mc_ver)
        .ok_or_else(|| anyhow!("MC version {} not found", mc_ver))?;
    let ver_text = client.get(&entry.url).send().await?.text().await?;
    let ver: VersionJson = serde_json::from_str(&ver_text)?;

    let ver_dir = shared_dir.join("versions").join(&mc_ver);
    fs::create_dir_all(&ver_dir)?;
    let ver_json_path = ver_dir.join(format!("{mc_ver}.json"));
    if !ver_json_path.exists() { fs::write(&ver_json_path, &ver_text)?; }

    let jar_path = ver_dir.join(format!("{mc_ver}.jar"));
    if !is_valid_file(&jar_path, ver.downloads.client.size) {
        progress(&app, "download", 14.0, "Downloading Minecraft client…");
        download_file(&client, &ver.downloads.client.url, &jar_path).await?;
    }

    // 3. Vanilla libraries
    let libs_dir     = shared_dir.join("libraries");
    let natives_dir  = ver_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;
    let mut classpath: Vec<String> = Vec::new();
    let total = ver.libraries.len();
    for (i, lib) in ver.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 10 == 0 {
            let pct = 16.0 + (i as f32 / total as f32) * 22.0;
            progress(&app, "download", pct, &format!("Vanilla libraries ({i}/{total})…"));
        }
        if !lib_allowed(lib) { continue; }
        let Some(dl) = &lib.downloads else { continue };
        if let Some(art) = &dl.artifact {
            let p = libs_dir.join(&art.path);
            fs::create_dir_all(p.parent().unwrap())?;
            if !is_valid_file(&p, art.size) { let _ = download_file(&client, &art.url, &p).await; }
            classpath.push(p.to_string_lossy().into_owned());
        }
        if let Some(natives_map) = &lib.natives {
            let key = os_classifier_key();
            if let Some(classifier) = natives_map.get(key) {
                let classifier = classifier.replace("${arch}", arch_bits());
                if let Some(classifiers) = &dl.classifiers {
                    if let Some(nat) = classifiers.get(&classifier) {
                        let p = libs_dir.join(&nat.path);
                        fs::create_dir_all(p.parent().unwrap())?;
                        if !is_valid_file(&p, nat.size) { let _ = download_file(&client, &nat.url, &p).await; }
                        let _ = extract_natives(&p, &natives_dir);
                    }
                }
            }
        }
    }

    // 4. Assets
    progress(&app, "download", 40.0, "Downloading asset index…");
    let idx_dir = shared_dir.join("assets").join("indexes");
    fs::create_dir_all(&idx_dir)?;
    let idx_path = idx_dir.join(format!("{}.json", ver.asset_index.id));
    if !idx_path.exists() {
        let t = client.get(&ver.asset_index.url).send().await?.text().await?;
        fs::write(&idx_path, &t)?;
    }
    let idx: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;
    let objs_dir  = shared_dir.join("assets").join("objects");
    let total_a   = idx.objects.len();
    progress(&app, "download", 42.0, &format!("Assets (0/{total_a})…"));
    download_assets_parallel(&app, &client, &idx.objects, &objs_dir, concurrent_downloads, 42.0, 20.0,
        cancel_dl.clone(), pause_dl.clone(), dl_bytes.clone()).await;
    if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
    progress(&app, "download", 62.0, &format!("Assets ({total_a}/{total_a})…"));

    // 5. Quilt Loader profile + libraries
    progress(&app, "download", 63.0, "Fetching Quilt Loader profile…");
    let quilt_url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        mc_ver, loader_ver
    );
    let quilt_text = client.get(&quilt_url).send().await?.text().await?;
    let quilt: FabricProfile = serde_json::from_str(&quilt_text)
        .map_err(|e| anyhow!("Quilt profile parse: {e}"))?;

    let total_ql = quilt.libraries.len();
    for (i, qlib) in quilt.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 5 == 0 {
            let pct = 65.0 + (i as f32 / total_ql.max(1) as f32) * 12.0;
            progress(&app, "download", pct, &format!("Quilt libraries ({i}/{total_ql})…"));
        }
        let parts: Vec<&str> = qlib.name.splitn(3, ':').collect();
        if parts.len() < 3 { continue; }
        let (group, artifact, version) = (parts[0], parts[1], parts[2]);
        let group_path = group.replace('.', "/");
        let jar_name   = format!("{artifact}-{version}.jar");
        let rel_path   = format!("{group_path}/{artifact}/{version}/{jar_name}");
        let dest       = libs_dir.join(&rel_path);
        if !dest.exists() {
            fs::create_dir_all(dest.parent().unwrap())?;
            // Quilt profile libraries default to quiltmc maven if the url field is empty/generic
            let base_url = if qlib.url.is_empty() {
                "https://maven.quiltmc.org/repository/release/".to_string()
            } else {
                qlib.url.clone()
            };
            let url = format!("{}/{rel_path}", base_url.trim_end_matches('/'));
            let _ = download_file(&client, &url, &dest).await;
        }
        classpath.push(dest.to_string_lossy().into_owned());
    }

    // MC jar at end of classpath
    classpath.push(jar_path.to_string_lossy().into_owned());

    // 6. Java
    progress(&app, "launch", 79.0, "Finding Java runtime…");
    let java = ensure_java(&app, &client, &shared_dir, ver.java_version.as_ref()).await
        .context("Failed to obtain Java runtime")?;

    // 7. Build command
    progress(&app, "launch", 95.0, "Starting Minecraft (Quilt)…");
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath_str = classpath.join(sep);
    let token     = if offline { "0".to_string() } else { access_token };
    let user_type = if offline { "offline" } else { "msa" };

    let vars: HashMap<&str, String> = HashMap::from([
        ("${auth_player_name}", username.clone()),
        ("${version_name}", format!("quilt-loader-{loader_ver}-{mc_ver}")),
        ("${game_directory}", game_dir.to_string_lossy().into_owned()),
        ("${assets_root}", shared_dir.join("assets").to_string_lossy().into_owned()),
        ("${assets_index_name}", ver.asset_index.id.clone()),
        ("${auth_uuid}", uuid.clone()),
        ("${auth_access_token}", token),
        ("${user_type}", user_type.to_string()),
        ("${version_type}", "release".to_string()),
        ("${user_properties}", "{}".to_string()),
        ("${natives_directory}", natives_dir.to_string_lossy().into_owned()),
        ("${launcher_name}", "MLBV".to_string()),
        ("${launcher_version}", "1.0".to_string()),
        ("${classpath}", classpath_str.clone()),
        ("${classpath_separator}", sep.to_string()),
        ("${resolution_width}", "854".to_string()),
        ("${resolution_height}", "480".to_string()),
    ]);
    let replace = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars { out = out.replace(k, v); }
        out
    };

    let mut cmd_args: Vec<String> = Vec::new();
    if let Some(fa) = &quilt.arguments {
        for v in &fa.jvm { if let Some(s) = v.as_str() { cmd_args.push(replace(s)); } }
    }
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.jvm { resolve_arg(arg, &replace, &mut cmd_args); }
    } else {
        cmd_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        cmd_args.push("-Dminecraft.launcher.brand=MLBV".to_string());
        cmd_args.push("-cp".to_string());
        cmd_args.push(classpath_str);
    }
    cmd_args.push(format!("-Xmx{}m", max_ram_mb));
    cmd_args.push("-Xms256m".to_string());
    cmd_args.push(quilt.main_class.clone());
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.game { resolve_arg(arg, &replace, &mut cmd_args); }
    } else if let Some(old) = &ver.minecraft_arguments {
        for part in old.split_whitespace() { cmd_args.push(replace(part)); }
    }

    let jvm_lines = app.state::<GameState>().jvm_lines.clone();
    jvm_lines.lock().unwrap().clear();
    let mut java_cmd = std::process::Command::new(&java);
    java_cmd.args(&cmd_args).current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = java_cmd
        .spawn()
        .with_context(|| format!("Failed to start Java from {:?}", java))?;
    if let Some(out) = child.stdout.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(out).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }
    if let Some(err) = child.stderr.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(err).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }

    progress(&app, "launch", 100.0, "Minecraft (Quilt) launched!");

    *app.state::<GameState>().child.lock().unwrap() = Some(child);
    let _ = app.emit("game-running", true);

    let app_mon      = app.clone();
    let game_dir_mon = game_dir.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let state = app_mon.state::<GameState>();
            let mut guard = state.child.lock().unwrap();
            match guard.as_mut() {
                Some(c) => { if let Ok(Some(status)) = c.try_wait() {
                    *guard = None; drop(guard);
                    if !status.success() {
                        let log_path = game_dir_mon.join("logs").join("latest.log");
                        let log_tail = {
                            let gs = app_mon.state::<GameState>();
                            let captured = gs.jvm_lines.lock().unwrap();
                            if !captured.is_empty() {
                                let total = captured.len();
                                captured[total.saturating_sub(80)..].join("\n")
                            } else {
                                std::fs::read_to_string(&log_path)
                                    .map(|s| { let v: Vec<&str> = s.lines().collect(); v[v.len().saturating_sub(80)..].join("\n") })
                                    .unwrap_or_else(|_| "No output captured.".into())
                            }
                        };
                        let _ = app_mon.emit("game-crashed", serde_json::json!({
                            "exitCode": status.code().unwrap_or(-1),
                            "log": log_tail,
                            "logPath": log_path.to_string_lossy().into_owned(),
                        }));
                    }
                    let _ = app_mon.emit("game-running", false);
                    break;
                }}
                None => break,
            }
        }
    });

    Ok(())
}

// ─── Forge launcher ───────────────────────────────────────────────────────────

pub async fn run_forge(
    app: tauri::AppHandle,
    mc_version: String,
    forge_version: String, // full like "1.20.1-47.3.11"
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<()> {
    let shared_dir = shared_data_dir();
    let game_dir   = instances_dir().join(&instance_name);
    let mods_dir   = game_dir.join("mods");
    fs::create_dir_all(&shared_dir)?;
    fs::create_dir_all(&game_dir)?;
    fs::create_dir_all(&mods_dir)?;

    let client = reqwest::Client::builder().user_agent("MLBV/1.0").build()?;

    let state = app.state::<GameState>();
    let cancel_dl = state.cancel_dl.clone();
    let pause_dl  = state.pause_dl.clone();
    let dl_bytes  = state.dl_bytes.clone();
    cancel_dl.store(false, Ordering::Relaxed);
    pause_dl.store(false, Ordering::Relaxed);
    dl_bytes.store(0, Ordering::Relaxed);

    let _speed_guard = {
        let b = dl_bytes.clone();
        let a = app.clone();
        let h = tokio::spawn(async move {
            let mut last = 0u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let cur = b.load(Ordering::Relaxed);
                let bps = cur.saturating_sub(last);
                last = cur;
                let _ = a.emit("download-speed", serde_json::json!({ "bps": bps }));
            }
        });
        AbortOnDrop(h)
    };

    // Forge installer names the version "{mcVer}-forge-{forgeOnlyVer}" e.g. "1.20.1-forge-47.3.11"
    let forge_only = forge_version.strip_prefix(&format!("{mc_version}-")).unwrap_or(&forge_version);
    let forge_ver_name = format!("{mc_version}-forge-{forge_only}");

    // Check if already installed (version JSON exists)
    let forge_ver_dir = shared_dir.join("versions").join(&forge_ver_name);
    let forge_ver_json_path = forge_ver_dir.join(format!("{forge_ver_name}.json"));

    if !forge_ver_json_path.exists() {
        // Download installer
        let installer_url = format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{v}/forge-{v}-installer.jar",
            v = forge_version
        );
        let installer_dir = shared_dir.join("forge-installers");
        fs::create_dir_all(&installer_dir)?;
        let installer_path = installer_dir.join(format!("forge-{forge_version}-installer.jar"));

        if !installer_path.exists() {
            progress(&app, "download", 10.0, "Downloading Forge installer…");
            download_file(&client, &installer_url, &installer_path).await
                .context("Failed to download Forge installer")?;
        }

        // Need Java to run installer (use Java 17 minimum for modern Forge)
        let java_req = JavaVersionReq { component: "java-runtime-gamma".to_string(), major_version: 17 };
        let java = ensure_java(&app, &client, &shared_dir, Some(&java_req)).await
            .context("Failed to find Java for Forge installer")?;

        progress(&app, "install", 30.0, "Installing Forge (this may take a minute)…");

        // Forge installer may require launcher_profiles.json to exist in the target dir
        let profiles_path = shared_dir.join("launcher_profiles.json");
        if !profiles_path.exists() {
            let _ = fs::write(&profiles_path,
                r#"{"profiles":{},"selectedProfile":"(Default)","authenticationDatabase":{},"clientToken":""}"#);
        }

        let java_c = java.clone();
        let installer_path_c = installer_path.clone();
        let shared_dir_c = shared_dir.clone();
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&java_c)
                // No -Djava.awt.headless — Forge installer may need AWT to start up
                .arg("-jar")
                .arg(&installer_path_c)
                .arg("--installClient")
                .arg(&shared_dir_c)
                .current_dir(&shared_dir_c)
                .output()
        }).await?
          .map_err(|e| anyhow!("Failed to spawn Forge installer: {e}"))?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}\n{stderr}");
            let tail: String = combined.lines().rev().take(40)
                .collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
            return Err(anyhow!(
                "Forge installer failed (exit {:?}):\n{}", output.status.code(), tail.trim()
            ));
        }
    }

    if !forge_ver_json_path.exists() {
        // Scan versions/ for any directory the installer may have created
        let found = fs::read_dir(shared_dir.join("versions"))
            .ok()
            .and_then(|rd| rd.flatten().find(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.contains("forge") && n.contains(&mc_version)
            }))
            .map(|e| e.file_name().to_string_lossy().into_owned());
        if let Some(actual_name) = found {
            return Err(anyhow!(
                "Forge installed to '{}' but expected '{}'. Check shared/versions/ manually.",
                actual_name, forge_ver_name
            ));
        }
        return Err(anyhow!("Forge version JSON not found after install: {:?}", forge_ver_json_path));
    }

    // Parse Forge overlay version JSON
    let forge_json_text = fs::read_to_string(&forge_ver_json_path)?;
    let forge_json: serde_json::Value = serde_json::from_str(&forge_json_text)
        .map_err(|e| anyhow!("Forge version JSON parse error: {e}"))?;

    let main_class = forge_json["mainClass"].as_str()
        .ok_or_else(|| anyhow!("No mainClass in Forge version JSON"))?
        .to_string();
    let inherits_from = forge_json["inheritsFrom"].as_str()
        .unwrap_or(&mc_version)
        .to_string();

    // Load vanilla MC (base version)
    progress(&app, "fetch", 40.0, "Fetching vanilla Minecraft metadata…");
    let mf: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await?.json().await?;
    let entry = mf.versions.iter().find(|v| v.id == inherits_from)
        .ok_or_else(|| anyhow!("MC version {} not found in manifest", inherits_from))?;
    let ver_text = client.get(&entry.url).send().await?.text().await?;
    let ver: VersionJson = serde_json::from_str(&ver_text)?;

    let ver_dir = shared_dir.join("versions").join(&inherits_from);
    fs::create_dir_all(&ver_dir)?;
    let ver_json_path = ver_dir.join(format!("{inherits_from}.json"));
    if !ver_json_path.exists() { fs::write(&ver_json_path, &ver_text)?; }

    let jar_path = ver_dir.join(format!("{inherits_from}.jar"));
    if !is_valid_file(&jar_path, ver.downloads.client.size) {
        progress(&app, "download", 44.0, "Downloading Minecraft client…");
        download_file(&client, &ver.downloads.client.url, &jar_path).await?;
    }

    // Vanilla libraries
    let libs_dir    = shared_dir.join("libraries");
    let natives_dir = ver_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;
    let mut classpath: Vec<String> = Vec::new();
    let total = ver.libraries.len();
    for (i, lib) in ver.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 10 == 0 {
            let pct = 46.0 + (i as f32 / total as f32) * 12.0;
            progress(&app, "download", pct, &format!("Vanilla libraries ({i}/{total})…"));
        }
        if !lib_allowed(lib) { continue; }
        let Some(dl) = &lib.downloads else { continue };
        if let Some(art) = &dl.artifact {
            let p = libs_dir.join(&art.path);
            fs::create_dir_all(p.parent().unwrap())?;
            if !is_valid_file(&p, art.size) { let _ = download_file(&client, &art.url, &p).await; }
            classpath.push(p.to_string_lossy().into_owned());
        }
        if let Some(natives_map) = &lib.natives {
            let key = os_classifier_key();
            if let Some(classifier) = natives_map.get(key) {
                let classifier = classifier.replace("${arch}", arch_bits());
                if let Some(classifiers) = &dl.classifiers {
                    if let Some(nat) = classifiers.get(&classifier) {
                        let p = libs_dir.join(&nat.path);
                        fs::create_dir_all(p.parent().unwrap())?;
                        if !is_valid_file(&p, nat.size) { let _ = download_file(&client, &nat.url, &p).await; }
                        let _ = extract_natives(&p, &natives_dir);
                    }
                }
            }
        }
    }

    // Assets
    progress(&app, "download", 60.0, "Downloading asset index…");
    let idx_dir = shared_dir.join("assets").join("indexes");
    fs::create_dir_all(&idx_dir)?;
    let idx_path = idx_dir.join(format!("{}.json", ver.asset_index.id));
    if !idx_path.exists() {
        let t = client.get(&ver.asset_index.url).send().await?.text().await?;
        fs::write(&idx_path, &t)?;
    }
    let idx: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;
    let objs_dir = shared_dir.join("assets").join("objects");
    let total_a  = idx.objects.len();
    progress(&app, "download", 62.0, &format!("Assets (0/{total_a})…"));
    download_assets_parallel(&app, &client, &idx.objects, &objs_dir, concurrent_downloads, 62.0, 15.0,
        cancel_dl.clone(), pause_dl.clone(), dl_bytes.clone()).await;
    if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }

    // Forge libraries (already downloaded by installer, but add to classpath)
    progress(&app, "download", 78.0, "Resolving Forge libraries…");
    if let Some(forge_libs) = forge_json["libraries"].as_array() {
        for lib_val in forge_libs {
            let name = lib_val["name"].as_str().unwrap_or("");
            if name.is_empty() { continue; }
            // Try downloads.artifact path first
            if let (Some(path), Some(url)) = (
                lib_val["downloads"]["artifact"]["path"].as_str(),
                lib_val["downloads"]["artifact"]["url"].as_str(),
            ) {
                if !url.is_empty() {
                    let p = libs_dir.join(path);
                    if !p.exists() {
                        fs::create_dir_all(p.parent().unwrap())?;
                        let _ = download_file(&client, url, &p).await;
                    }
                    if p.exists() { classpath.push(p.to_string_lossy().into_owned()); }
                    continue;
                }
            }
            // Fallback: Maven coords
            let parts: Vec<&str> = name.splitn(3, ':').collect();
            if parts.len() >= 3 {
                let group_path = parts[0].replace('.', "/");
                let jar_name   = format!("{}-{}.jar", parts[1], parts[2]);
                let p = libs_dir.join(&group_path).join(parts[1]).join(parts[2]).join(&jar_name);
                if p.exists() { classpath.push(p.to_string_lossy().into_owned()); }
            }
        }
    }

    // MC jar at end of classpath
    classpath.push(jar_path.to_string_lossy().into_owned());

    // Java
    progress(&app, "launch", 88.0, "Finding Java runtime…");
    let java = ensure_java(&app, &client, &shared_dir, ver.java_version.as_ref()).await
        .context("Failed to obtain Java runtime")?;

    // Build command
    progress(&app, "launch", 95.0, "Starting Minecraft (Forge)…");
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath_str = classpath.join(sep);
    let token     = if offline { "0".to_string() } else { access_token };
    let user_type = if offline { "offline" } else { "msa" };

    let vars: HashMap<&str, String> = HashMap::from([
        ("${auth_player_name}", username.clone()),
        ("${version_name}", forge_ver_name.clone()),
        ("${game_directory}", game_dir.to_string_lossy().into_owned()),
        ("${assets_root}", shared_dir.join("assets").to_string_lossy().into_owned()),
        ("${assets_index_name}", ver.asset_index.id.clone()),
        ("${auth_uuid}", uuid.clone()),
        ("${auth_access_token}", token),
        ("${user_type}", user_type.to_string()),
        ("${version_type}", "release".to_string()),
        ("${user_properties}", "{}".to_string()),
        ("${natives_directory}", natives_dir.to_string_lossy().into_owned()),
        ("${launcher_name}", "MLBV".to_string()),
        ("${launcher_version}", "1.0".to_string()),
        ("${classpath}", classpath_str.clone()),
        ("${classpath_separator}", sep.to_string()),
        ("${library_directory}", libs_dir.to_string_lossy().into_owned()),
        ("${resolution_width}", "854".to_string()),
        ("${resolution_height}", "480".to_string()),
    ]);
    let replace = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars { out = out.replace(k, v); }
        out
    };

    let mut cmd_args: Vec<String> = Vec::new();

    // Forge JVM args first (includes module path, DlibraryDirectory, etc.)
    if let Some(forge_args) = forge_json["arguments"]["jvm"].as_array() {
        for v in forge_args {
            if let Some(s) = v.as_str() {
                cmd_args.push(replace(s));
            } else if v.is_object() {
                // Conditional arg — parse and apply rule
                let rules = v["rules"].as_array();
                let allowed = rules.map_or(true, |rs| rs.iter().any(|r| {
                    r["action"].as_str() == Some("allow") && {
                        let os_name = r["os"]["name"].as_str().unwrap_or("");
                        os_name.is_empty()
                            || (cfg!(windows) && os_name == "windows")
                            || (cfg!(target_os = "macos") && os_name == "osx")
                            || (cfg!(target_os = "linux") && os_name == "linux")
                    }
                }));
                if allowed {
                    match &v["value"] {
                        serde_json::Value::String(s) => cmd_args.push(replace(s)),
                        serde_json::Value::Array(arr) => {
                            for s in arr.iter().filter_map(|x| x.as_str()) { cmd_args.push(replace(s)); }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    // Vanilla JVM args (provides -cp ${classpath} and natives path for old Forge / fallback)
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.jvm { resolve_arg(arg, &replace, &mut cmd_args); }
    } else {
        cmd_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        cmd_args.push("-Dminecraft.launcher.brand=MLBV".to_string());
        cmd_args.push("-cp".to_string());
        cmd_args.push(classpath_str.clone());
    }
    cmd_args.push(format!("-Xmx{}m", max_ram_mb));
    cmd_args.push("-Xms256m".to_string());
    cmd_args.push(main_class.clone());

    // Game args
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.game { resolve_arg(arg, &replace, &mut cmd_args); }
    } else if let Some(old) = &ver.minecraft_arguments {
        for part in old.split_whitespace() { cmd_args.push(replace(part)); }
    }
    // Forge overlay game args
    if let Some(forge_game_args) = forge_json["arguments"]["game"].as_array() {
        for v in forge_game_args {
            if let Some(s) = v.as_str() { cmd_args.push(replace(s)); }
        }
    } else if let Some(old) = forge_json["minecraftArguments"].as_str() {
        for part in old.split_whitespace() { cmd_args.push(replace(part)); }
    }

    let jvm_lines = app.state::<GameState>().jvm_lines.clone();
    jvm_lines.lock().unwrap().clear();
    let mut java_cmd = std::process::Command::new(&java);
    java_cmd.args(&cmd_args).current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = java_cmd
        .spawn()
        .with_context(|| format!("Failed to start Java from {:?}", java))?;
    if let Some(out) = child.stdout.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(out).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }
    if let Some(err) = child.stderr.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(err).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }

    progress(&app, "launch", 100.0, "Minecraft (Forge) launched!");
    *app.state::<GameState>().child.lock().unwrap() = Some(child);
    let _ = app.emit("game-running", true);

    let app_mon      = app.clone();
    let game_dir_mon = game_dir.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let state = app_mon.state::<GameState>();
            let mut guard = state.child.lock().unwrap();
            match guard.as_mut() {
                Some(c) => { if let Ok(Some(status)) = c.try_wait() {
                    *guard = None; drop(guard);
                    if !status.success() {
                        let log_path = game_dir_mon.join("logs").join("latest.log");
                        let log_tail = {
                            let gs = app_mon.state::<GameState>();
                            let captured = gs.jvm_lines.lock().unwrap();
                            if !captured.is_empty() {
                                let total = captured.len();
                                captured[total.saturating_sub(80)..].join("\n")
                            } else {
                                std::fs::read_to_string(&log_path)
                                    .map(|s| { let v: Vec<&str> = s.lines().collect(); v[v.len().saturating_sub(80)..].join("\n") })
                                    .unwrap_or_else(|_| "No output captured.".into())
                            }
                        };
                        let _ = app_mon.emit("game-crashed", serde_json::json!({
                            "exitCode": status.code().unwrap_or(-1),
                            "log": log_tail,
                            "logPath": log_path.to_string_lossy().into_owned(),
                        }));
                    }
                    let _ = app_mon.emit("game-running", false);
                    break;
                }}
                None => break,
            }
        }
    });

    Ok(())
}

// ─── NeoForge launcher ────────────────────────────────────────────────────────

pub async fn run_neoforge(
    app: tauri::AppHandle,
    mc_version: String,
    neoforge_version: String, // like "21.1.172"
    instance_name: String,
    username: String,
    uuid: String,
    offline: bool,
    access_token: String,
    concurrent_downloads: u32,
    max_ram_mb: u32,
) -> Result<()> {
    let shared_dir = shared_data_dir();
    let game_dir   = instances_dir().join(&instance_name);
    let mods_dir   = game_dir.join("mods");
    fs::create_dir_all(&shared_dir)?;
    fs::create_dir_all(&game_dir)?;
    fs::create_dir_all(&mods_dir)?;

    let client = reqwest::Client::builder().user_agent("MLBV/1.0").build()?;

    let state = app.state::<GameState>();
    let cancel_dl = state.cancel_dl.clone();
    let pause_dl  = state.pause_dl.clone();
    let dl_bytes  = state.dl_bytes.clone();
    cancel_dl.store(false, Ordering::Relaxed);
    pause_dl.store(false, Ordering::Relaxed);
    dl_bytes.store(0, Ordering::Relaxed);

    let _speed_guard = {
        let b = dl_bytes.clone();
        let a = app.clone();
        let h = tokio::spawn(async move {
            let mut last = 0u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let cur = b.load(Ordering::Relaxed);
                let bps = cur.saturating_sub(last);
                last = cur;
                let _ = a.emit("download-speed", serde_json::json!({ "bps": bps }));
            }
        });
        AbortOnDrop(h)
    };

    let neo_ver_name = format!("neoforge-{}", neoforge_version);

    let neo_ver_dir = shared_dir.join("versions").join(&neo_ver_name);
    let neo_ver_json_path = neo_ver_dir.join(format!("{neo_ver_name}.json"));

    if !neo_ver_json_path.exists() {
        let installer_url = format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{v}/neoforge-{v}-installer.jar",
            v = neoforge_version
        );
        let installer_dir = shared_dir.join("neoforge-installers");
        fs::create_dir_all(&installer_dir)?;
        let installer_path = installer_dir.join(format!("neoforge-{neoforge_version}-installer.jar"));

        if !installer_path.exists() {
            progress(&app, "download", 10.0, "Downloading NeoForge installer…");
            download_file(&client, &installer_url, &installer_path).await
                .context("Failed to download NeoForge installer")?;
        }

        let java_req = JavaVersionReq { component: "java-runtime-gamma".to_string(), major_version: 21 };
        let java = ensure_java(&app, &client, &shared_dir, Some(&java_req)).await
            .context("Failed to find Java for NeoForge installer")?;

        progress(&app, "install", 30.0, "Installing NeoForge (this may take a minute)…");

        let profiles_path = shared_dir.join("launcher_profiles.json");
        if !profiles_path.exists() {
            let _ = fs::write(&profiles_path,
                r#"{"profiles":{},"selectedProfile":"(Default)","authenticationDatabase":{},"clientToken":""}"#);
        }

        let java_c = java.clone();
        let installer_path_c = installer_path.clone();
        let shared_dir_c = shared_dir.clone();
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&java_c)
                .arg("-jar")
                .arg(&installer_path_c)
                .arg("--installClient")
                .arg(&shared_dir_c)
                .current_dir(&shared_dir_c)
                .output()
        }).await?
          .map_err(|e| anyhow!("Failed to spawn NeoForge installer: {e}"))?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}\n{stderr}");
            let tail: String = combined.lines().rev().take(40)
                .collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
            return Err(anyhow!(
                "NeoForge installer failed (exit {:?}):\n{}", output.status.code(), tail.trim()
            ));
        }
    }

    if !neo_ver_json_path.exists() {
        let found = fs::read_dir(shared_dir.join("versions"))
            .ok()
            .and_then(|rd| rd.flatten().find(|e| {
                e.file_name().to_string_lossy().contains("neoforge")
            }))
            .map(|e| e.file_name().to_string_lossy().into_owned());
        if let Some(actual_name) = found {
            return Err(anyhow!(
                "NeoForge installed to '{}' but expected '{}'. Check shared/versions/ manually.",
                actual_name, neo_ver_name
            ));
        }
        return Err(anyhow!("NeoForge version JSON not found after install: {:?}", neo_ver_json_path));
    }

    let neo_json_text = fs::read_to_string(&neo_ver_json_path)?;
    let neo_json: serde_json::Value = serde_json::from_str(&neo_json_text)
        .map_err(|e| anyhow!("NeoForge version JSON parse: {e}"))?;

    let main_class = neo_json["mainClass"].as_str()
        .ok_or_else(|| anyhow!("No mainClass in NeoForge version JSON"))?
        .to_string();
    let inherits_from = neo_json["inheritsFrom"].as_str()
        .unwrap_or(&mc_version)
        .to_string();

    // Load vanilla MC
    progress(&app, "fetch", 40.0, "Fetching vanilla Minecraft metadata…");
    let mf: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await?.json().await?;
    let entry = mf.versions.iter().find(|v| v.id == inherits_from)
        .ok_or_else(|| anyhow!("MC version {} not found in manifest", inherits_from))?;
    let ver_text = client.get(&entry.url).send().await?.text().await?;
    let ver: VersionJson = serde_json::from_str(&ver_text)?;

    let ver_dir = shared_dir.join("versions").join(&inherits_from);
    fs::create_dir_all(&ver_dir)?;
    let ver_json_path = ver_dir.join(format!("{inherits_from}.json"));
    if !ver_json_path.exists() { fs::write(&ver_json_path, &ver_text)?; }

    let jar_path = ver_dir.join(format!("{inherits_from}.jar"));
    if !is_valid_file(&jar_path, ver.downloads.client.size) {
        progress(&app, "download", 44.0, "Downloading Minecraft client…");
        download_file(&client, &ver.downloads.client.url, &jar_path).await?;
    }

    let libs_dir    = shared_dir.join("libraries");
    let natives_dir = ver_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;
    let mut classpath: Vec<String> = Vec::new();

    let total = ver.libraries.len();
    for (i, lib) in ver.libraries.iter().enumerate() {
        if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
        while pause_dl.load(Ordering::Relaxed) {
            if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if i % 10 == 0 {
            let pct = 46.0 + (i as f32 / total as f32) * 12.0;
            progress(&app, "download", pct, &format!("Vanilla libraries ({i}/{total})…"));
        }
        if !lib_allowed(lib) { continue; }
        let Some(dl) = &lib.downloads else { continue };
        if let Some(art) = &dl.artifact {
            let p = libs_dir.join(&art.path);
            fs::create_dir_all(p.parent().unwrap())?;
            if !is_valid_file(&p, art.size) { let _ = download_file(&client, &art.url, &p).await; }
            classpath.push(p.to_string_lossy().into_owned());
        }
        if let Some(natives_map) = &lib.natives {
            let key = os_classifier_key();
            if let Some(classifier) = natives_map.get(key) {
                let classifier = classifier.replace("${arch}", arch_bits());
                if let Some(classifiers) = &dl.classifiers {
                    if let Some(nat) = classifiers.get(&classifier) {
                        let p = libs_dir.join(&nat.path);
                        fs::create_dir_all(p.parent().unwrap())?;
                        if !is_valid_file(&p, nat.size) { let _ = download_file(&client, &nat.url, &p).await; }
                        let _ = extract_natives(&p, &natives_dir);
                    }
                }
            }
        }
    }

    // Assets
    progress(&app, "download", 60.0, "Downloading asset index…");
    let idx_dir = shared_dir.join("assets").join("indexes");
    fs::create_dir_all(&idx_dir)?;
    let idx_path = idx_dir.join(format!("{}.json", ver.asset_index.id));
    if !idx_path.exists() {
        let t = client.get(&ver.asset_index.url).send().await?.text().await?;
        fs::write(&idx_path, &t)?;
    }
    let idx: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;
    let objs_dir = shared_dir.join("assets").join("objects");
    let total_a  = idx.objects.len();
    progress(&app, "download", 62.0, &format!("Assets (0/{total_a})…"));
    download_assets_parallel(&app, &client, &idx.objects, &objs_dir, concurrent_downloads, 62.0, 15.0,
        cancel_dl.clone(), pause_dl.clone(), dl_bytes.clone()).await;
    if cancel_dl.load(Ordering::Relaxed) { return Err(anyhow!("Download cancelled")); }

    // NeoForge libraries
    progress(&app, "download", 78.0, "Resolving NeoForge libraries…");
    if let Some(neo_libs) = neo_json["libraries"].as_array() {
        for lib_val in neo_libs {
            let name = lib_val["name"].as_str().unwrap_or("");
            if name.is_empty() { continue; }
            if let (Some(path), Some(url)) = (
                lib_val["downloads"]["artifact"]["path"].as_str(),
                lib_val["downloads"]["artifact"]["url"].as_str(),
            ) {
                if !url.is_empty() {
                    let p = libs_dir.join(path);
                    if !p.exists() {
                        fs::create_dir_all(p.parent().unwrap())?;
                        let _ = download_file(&client, url, &p).await;
                    }
                    if p.exists() { classpath.push(p.to_string_lossy().into_owned()); }
                    continue;
                }
            }
            let parts: Vec<&str> = name.splitn(3, ':').collect();
            if parts.len() >= 3 {
                let group_path = parts[0].replace('.', "/");
                let jar_name   = format!("{}-{}.jar", parts[1], parts[2]);
                let p = libs_dir.join(&group_path).join(parts[1]).join(parts[2]).join(&jar_name);
                if p.exists() { classpath.push(p.to_string_lossy().into_owned()); }
            }
        }
    }

    classpath.push(jar_path.to_string_lossy().into_owned());

    // Java
    progress(&app, "launch", 88.0, "Finding Java runtime…");
    let java = ensure_java(&app, &client, &shared_dir, ver.java_version.as_ref()).await
        .context("Failed to obtain Java runtime")?;

    progress(&app, "launch", 95.0, "Starting Minecraft (NeoForge)…");
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath_str = classpath.join(sep);
    let token     = if offline { "0".to_string() } else { access_token };
    let user_type = if offline { "offline" } else { "msa" };

    let vars: HashMap<&str, String> = HashMap::from([
        ("${auth_player_name}", username.clone()),
        ("${version_name}", neo_ver_name.clone()),
        ("${game_directory}", game_dir.to_string_lossy().into_owned()),
        ("${assets_root}", shared_dir.join("assets").to_string_lossy().into_owned()),
        ("${assets_index_name}", ver.asset_index.id.clone()),
        ("${auth_uuid}", uuid.clone()),
        ("${auth_access_token}", token),
        ("${user_type}", user_type.to_string()),
        ("${version_type}", "release".to_string()),
        ("${user_properties}", "{}".to_string()),
        ("${natives_directory}", natives_dir.to_string_lossy().into_owned()),
        ("${launcher_name}", "MLBV".to_string()),
        ("${launcher_version}", "1.0".to_string()),
        ("${classpath}", classpath_str.clone()),
        ("${classpath_separator}", sep.to_string()),
        ("${library_directory}", libs_dir.to_string_lossy().into_owned()),
        ("${resolution_width}", "854".to_string()),
        ("${resolution_height}", "480".to_string()),
    ]);
    let replace = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars { out = out.replace(k, v); }
        out
    };

    let mut cmd_args: Vec<String> = Vec::new();
    // NeoForge JVM args first (includes module path, DlibraryDirectory, etc.)
    if let Some(neo_jvm) = neo_json["arguments"]["jvm"].as_array() {
        for v in neo_jvm {
            if let Some(s) = v.as_str() {
                cmd_args.push(replace(s));
            } else if v.is_object() {
                let rules = v["rules"].as_array();
                let allowed = rules.map_or(true, |rs| rs.iter().any(|r| {
                    r["action"].as_str() == Some("allow") && {
                        let os_name = r["os"]["name"].as_str().unwrap_or("");
                        os_name.is_empty()
                            || (cfg!(windows) && os_name == "windows")
                            || (cfg!(target_os = "macos") && os_name == "osx")
                            || (cfg!(target_os = "linux") && os_name == "linux")
                    }
                }));
                if allowed {
                    match &v["value"] {
                        serde_json::Value::String(s) => cmd_args.push(replace(s)),
                        serde_json::Value::Array(arr) => {
                            for s in arr.iter().filter_map(|x| x.as_str()) { cmd_args.push(replace(s)); }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    // Vanilla JVM args (provides -cp ${classpath} and natives path for fallback)
    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.jvm { resolve_arg(arg, &replace, &mut cmd_args); }
    } else {
        cmd_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        cmd_args.push("-Dminecraft.launcher.brand=MLBV".to_string());
        cmd_args.push("-cp".to_string());
        cmd_args.push(classpath_str.clone());
    }
    cmd_args.push(format!("-Xmx{}m", max_ram_mb));
    cmd_args.push("-Xms256m".to_string());
    cmd_args.push(main_class.clone());

    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.game { resolve_arg(arg, &replace, &mut cmd_args); }
    } else if let Some(old) = &ver.minecraft_arguments {
        for part in old.split_whitespace() { cmd_args.push(replace(part)); }
    }
    if let Some(neo_game) = neo_json["arguments"]["game"].as_array() {
        for v in neo_game {
            if let Some(s) = v.as_str() { cmd_args.push(replace(s)); }
        }
    } else if let Some(old) = neo_json["minecraftArguments"].as_str() {
        for part in old.split_whitespace() { cmd_args.push(replace(part)); }
    }

    let jvm_lines = app.state::<GameState>().jvm_lines.clone();
    jvm_lines.lock().unwrap().clear();
    let mut java_cmd = std::process::Command::new(&java);
    java_cmd.args(&cmd_args).current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = java_cmd
        .spawn()
        .with_context(|| format!("Failed to start Java from {:?}", java))?;
    if let Some(out) = child.stdout.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(out).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }
    if let Some(err) = child.stderr.take() {
        let buf = jvm_lines.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            std::io::BufReader::new(err).lines().flatten().for_each(|l| { buf.lock().unwrap().push(l); });
        });
    }

    progress(&app, "launch", 100.0, "Minecraft (NeoForge) launched!");
    *app.state::<GameState>().child.lock().unwrap() = Some(child);
    let _ = app.emit("game-running", true);

    let app_mon      = app.clone();
    let game_dir_mon = game_dir.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let state = app_mon.state::<GameState>();
            let mut guard = state.child.lock().unwrap();
            match guard.as_mut() {
                Some(c) => { if let Ok(Some(status)) = c.try_wait() {
                    *guard = None; drop(guard);
                    if !status.success() {
                        let log_path = game_dir_mon.join("logs").join("latest.log");
                        let log_tail = {
                            let gs = app_mon.state::<GameState>();
                            let captured = gs.jvm_lines.lock().unwrap();
                            if !captured.is_empty() {
                                let total = captured.len();
                                captured[total.saturating_sub(80)..].join("\n")
                            } else {
                                std::fs::read_to_string(&log_path)
                                    .map(|s| { let v: Vec<&str> = s.lines().collect(); v[v.len().saturating_sub(80)..].join("\n") })
                                    .unwrap_or_else(|_| "No output captured.".into())
                            }
                        };
                        let _ = app_mon.emit("game-crashed", serde_json::json!({
                            "exitCode": status.code().unwrap_or(-1),
                            "log": log_tail,
                            "logPath": log_path.to_string_lossy().into_owned(),
                        }));
                    }
                    let _ = app_mon.emit("game-running", false);
                    break;
                }}
                None => break,
            }
        }
    });

    Ok(())
}

// ─── LB mod WebView download ──────────────────────────────────────────────────

/// Open the LiquidBounce download-queue page in a popup.
/// Uses Tauri's on_download to intercept the browser-initiated file download
/// (WebView2 fires a download event, NOT a navigation, for Content-Disposition:attachment).
/// Redirects the save path to a temp file, polls size for progress, extracts the JAR.
async fn download_lb_mod_webview(
    app: &tauri::AppHandle,
    queue_url: &str,
    dest: &PathBuf,
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
        .map_err(|e| anyhow!("Неверный URL: {e}"))?;

    let win = tauri::WebviewWindowBuilder::new(app, "lb-dl", tauri::WebviewUrl::External(parsed))
        .title("LiquidBounce — нажмите «Download»")
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
        .map_err(|e| anyhow!("Не удалось открыть окно: {e}"))?;

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
            return Err(anyhow!("Ошибка при скачивании мода в браузере"));
        }
        // Window closed before the download even started → user cancelled
        if closed.load(Ordering::Relaxed) && !started.load(Ordering::Relaxed) {
            return Err(anyhow!("Скачивание мода отменено пользователем"));
        }

        // Show file-size progress while WebView downloads
        if started.load(Ordering::Relaxed) {
            let sz_mb = fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0) / 1024 / 1024;
            let pct   = (77.0_f32 + sz_mb as f32).min(88.0);
            progress(app, "download", pct, &format!("Скачиваю LiquidBounce… {sz_mb} MB"));
        }

        if start.elapsed().as_secs() > 300 {
            let _ = win.close();
            return Err(anyhow!("Время ожидания истекло (5 мин)"));
        }
    }
    let _ = win.close();

    // Read the completed download and process it
    let bytes = fs::read(&temp_path).context("Чтение скачанного файла")?;
    let _ = fs::remove_file(&temp_path); // cleanup regardless

    if bytes.len() < 4 || !bytes.starts_with(b"PK") {
        return Err(anyhow!("Скачанный файл не является ZIP/JAR"));
    }

    // Try to extract a .jar from inside the ZIP; if none found, the file itself is the JAR
    match extract_jar_from_zip(&bytes, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::write(dest, &bytes).context("Сохранение JAR")?;
            Ok(())
        }
    }
}

/// Extract the first .jar found inside a ZIP archive.
fn extract_jar_from_zip(zip_bytes: &[u8], dest: &PathBuf) -> Result<()> {
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

    let idx = jar_idx.ok_or_else(|| anyhow!("В ZIP нет .jar файла"))?;
    let mut entry = archive.by_index(idx)?;
    let mut data  = Vec::new();
    entry.read_to_end(&mut data)?;
    fs::write(dest, data)?;
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn mc_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".minecraft")
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(if cfg!(target_os = "macos") {
                "Library/Application Support/minecraft"
            } else {
                ".minecraft"
            })
    }
}

fn is_valid_file(path: &PathBuf, expected_size: u64) -> bool {
    if expected_size == 0 { return path.exists(); }
    match fs::metadata(path) {
        Ok(m) => m.len() == expected_size,
        Err(_) => false,
    }
}

async fn download_file(client: &reqwest::Client, url: &str, path: &PathBuf) -> Result<()> {
    let bytes = client.get(url).send().await?.bytes().await?;
    fs::write(path, &bytes)?;
    Ok(())
}

fn extract_natives(zip_path: &PathBuf, dest: &PathBuf) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.starts_with("META-INF") || name.ends_with('/') { continue; }
        let out = dest.join(&name);
        if let Some(p) = out.parent() { fs::create_dir_all(p)?; }
        let mut f = fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
    }
    Ok(())
}

fn os_condition_matches(os: &OsCondition) -> bool {
    let name_ok = match &os.name {
        None => true,
        Some(n) => (cfg!(windows) && n == "windows") ||
                   (cfg!(target_os = "macos") && n == "osx") ||
                   (cfg!(target_os = "linux") && n == "linux"),
    };
    let arch_ok = match &os.arch {
        None => true,
        Some(a) => match a.as_str() {
            // "x86" means 32-bit only — exclude on 64-bit JVM (avoids UnsatisfiedLinkError)
            "x86" => cfg!(target_pointer_width = "32"),
            "x86_64" | "amd64" => cfg!(target_pointer_width = "64"),
            _ => true,
        },
    };
    name_ok && arch_ok
}

fn lib_allowed(lib: &Library) -> bool {
    if lib.rules.is_empty() { return true; }
    let mut allowed = false;
    for rule in &lib.rules {
        let os_match = match &rule.os {
            None => true,
            Some(os) => os_condition_matches(os),
        };
        if os_match { allowed = rule.action == "allow"; }
    }
    allowed
}

fn resolve_arg(arg: &Arg, replace: &impl Fn(&str) -> String, out: &mut Vec<String>) {
    match arg {
        Arg::Plain(s) => out.push(replace(s)),
        Arg::Conditional { rules, value } => {
            // Skip args that require specific features (demo, custom resolution)
            if rules.iter().any(|r| r.features.is_some()) { return; }

            let mut allowed = false;
            for rule in rules {
                let os_match = match &rule.os {
                    None => true,
                    Some(os) => os_condition_matches(os),
                };
                if os_match { allowed = rule.action == "allow"; }
            }
            if !allowed { return; }

            match value {
                ArgValue::One(s) => out.push(replace(s)),
                ArgValue::Many(v) => out.extend(v.iter().map(|s| replace(s))),
            }
        }
    }
}

fn os_classifier_key() -> &'static str {
    if cfg!(windows) { "windows" }
    else if cfg!(target_os = "macos") { "osx" }
    else { "linux" }
}

fn arch_bits() -> &'static str {
    if cfg!(target_pointer_width = "64") { "64" } else { "32" }
}

// ─── Java helpers ─────────────────────────────────────────────────────────────

pub fn folder_java_major(name: &str) -> Option<u32> {
    name.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .find(|&n| n >= 8)
}

pub fn find_java_exe_recursive(dir: &PathBuf, exe: &str) -> Option<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return None };
    let mut subdirs = vec![];
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.file_name().map(|n| n == exe).unwrap_or(false) {
            return Some(path);
        }
    }
    for sub in subdirs {
        if let Some(found) = find_java_exe_recursive(&sub, exe) {
            return Some(found);
        }
    }
    None
}

fn find_java(root: &PathBuf, req: Option<&JavaVersionReq>) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "javaw.exe" } else { "java" };
    let req_major = req.map(|r| r.major_version);

    // Exact match for mlbv-managed JREs (prevents Java 25 being used when Java 21 is needed)
    let ver_exact  = |detected: Option<u32>| match (detected, req_major) {
        (Some(v), Some(r)) => v == r,
        (None, Some(_))    => false,
        (_, None)          => true,
    };
    // Compatible (>=) for system-installed Java we don't control
    let ver_compat = |detected: Option<u32>| match (detected, req_major) {
        (Some(v), Some(r)) => v >= r,
        (None, Some(_))    => false,
        (_, None)          => true,
    };

    // 1. mlbv's own downloaded JREs (shared/java/jre-{N}/) — exact version required
    let mlbv_java = root.join("java");
    if mlbv_java.exists() {
        if let Ok(entries) = fs::read_dir(&mlbv_java) {
            let mut dirs: Vec<_> = entries.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .collect();
            dirs.sort_by_key(|e| folder_java_major(&e.file_name().to_string_lossy()).unwrap_or(0));
            for entry in dirs.iter() {
                let ver = folder_java_major(&entry.file_name().to_string_lossy());
                if ver_exact(ver) {
                    if let Some(java) = find_java_exe_recursive(&entry.path(), exe) {
                        return Some(java);
                    }
                }
            }
        }
    }

    // 2. Minecraft's own bundled runtime
    if let Some(req) = req {
        let component = &req.component;
        let rt_base = root.join("runtime").join(component);
        let platforms: &[&str] = if cfg!(windows) {
            &["windows-x64", "windows-x86", "windows"]
        } else if cfg!(target_os = "macos") {
            &["mac-os", "mac-os-arm64"]
        } else {
            &["linux", "linux-i386"]
        };
        for platform in platforms {
            let java = rt_base.join(platform).join(component).join("bin").join(exe);
            if java.exists() { return Some(java); }
            let java_mac = rt_base.join(platform).join(component)
                .join("jre.bundle/Contents/Home/bin/java");
            if java_mac.exists() { return Some(java_mac); }
        }
    }

    // 3. JAVA_HOME — version from last path component
    if let Ok(home) = std::env::var("JAVA_HOME") {
        let home_path = PathBuf::from(&home);
        let p = home_path.join("bin").join(exe);
        if p.exists() {
            let ver = home_path.file_name()
                .and_then(|n| folder_java_major(&n.to_string_lossy()));
            if ver_compat(ver) { return Some(p); }
        }
    }

    // 4. Common install paths — pick lowest satisfying version
    #[cfg(windows)]
    {
        let bases = [
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\Microsoft",
            "C:\\Program Files\\Amazon Corretto",
            "C:\\Program Files\\Java",
            "C:\\Program Files (x86)\\Java",
        ];
        for base in &bases {
            if let Ok(rd) = std::fs::read_dir(base) {
                let mut dirs: Vec<_> = rd.flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .collect();
                dirs.sort_by_key(|e| folder_java_major(&e.file_name().to_string_lossy()).unwrap_or(0));
                for entry in dirs.iter() {
                    let ver = folder_java_major(&entry.file_name().to_string_lossy());
                    let p = entry.path().join("bin").join(exe);
                    if p.exists() && ver_compat(ver) { return Some(p); }
                }
            }
        }
    }

    // 5. PATH fallback (any version if no requirement)
    if req_major.is_none()
        && std::process::Command::new(exe).arg("-version").output().is_ok()
    {
        return Some(PathBuf::from(exe));
    }

    None
}

pub async fn ensure_java(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    shared_dir: &PathBuf,
    req: Option<&JavaVersionReq>,
) -> Result<PathBuf> {
    let major = req.map(|r| r.major_version).unwrap_or(21);
    let exe   = if cfg!(windows) { "javaw.exe" } else { "java" };

    progress(app, "launch", 88.0, &format!("Java {major}: scanning local installs (shared_dir={})…", shared_dir.display()));
    if let Some(java) = find_java(shared_dir, req).or_else(|| find_java(&mc_dir(), req)) {
        progress(app, "launch", 88.5, &format!("Java {major}: found at {}", java.display()));
        return Ok(java);
    }

    let java_dir = shared_dir.join("java").join(format!("jre-{major}"));
    progress(app, "launch", 88.5, &format!("Java {major}: not found locally, checking {}", java_dir.display()));

    // Already extracted from a previous download?
    if java_dir.exists() {
        progress(app, "launch", 88.5, &format!("Java {major}: dir exists, scanning for {exe}…"));
        if let Some(found) = find_java_exe_recursive(&java_dir, exe) {
            return Ok(found);
        }
        progress(app, "launch", 88.5, &format!("Java {major}: dir exists but no {exe} found inside"));
    }

    // Auto-download Eclipse Temurin JRE via Adoptium v3 feature_releases API
    // Note: /assets/latest/ is broken (404); /assets/feature_releases/ works and uses binaries[] array
    let os_str   = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch_str = match std::env::consts::ARCH { "x86_64" => "x64", "aarch64" => "aarch64", _ => "x86" };
    let assets_url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{major}/ga?architecture={arch_str}&heap_size=normal&image_type=jre&os={os_str}&vendor=eclipse&page_size=1"
    );
    progress(app, "download", 88.0, &format!("Java {major}: querying Adoptium (os={os_str} arch={arch_str})…"));

    let api_resp = client
        .get(&assets_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot reach Adoptium API (adoptium.net)")?;
    let api_status = api_resp.status();
    let assets_json: serde_json::Value = api_resp.json().await
        .with_context(|| format!("Adoptium assets JSON parse failed (HTTP {api_status}, url={assets_url})"))?;

    progress(app, "download", 88.2, &format!("Java {major}: Adoptium replied HTTP {api_status}, entries={}", assets_json.as_array().map(|a| a.len()).unwrap_or(0)));

    let direct_url = assets_json[0]["binaries"][0]["package"]["link"]
        .as_str()
        .ok_or_else(|| anyhow!(
            "Java {major} not found in Adoptium catalog (HTTP {api_status}, entries={}, url={assets_url}) — install manually: https://adoptium.net",
            assets_json.as_array().map(|a| a.len()).unwrap_or(0)
        ))?
        .to_string();

    progress(app, "download", 88.3, &format!("Java {major}: got CDN url, downloading…"));

    let resp = client.get(&direct_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot download Java from Adoptium CDN")?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Java {major} download failed (HTTP {})",
            resp.status()
        ));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut bytes: Vec<u8> = Vec::new();
    let mut downloaded: u64 = 0;
    let mut last_mb: u64 = 0;
    let mut resp = resp;
    while let Some(chunk) = resp.chunk().await.context("Java download interrupted")? {
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let mb = downloaded / 1_048_576;
        if mb > last_mb {
            last_mb = mb;
            if total > 0 {
                let tot = total / 1_048_576;
                let pct = 88.0_f32 + (downloaded as f32 / total as f32) * 5.0;
                progress(app, "download", pct, &format!("Java {major}: {mb}/{tot} MB…"));
            } else {
                progress(app, "download", 89.0, &format!("Java {major}: {mb} MB…"));
            }
        }
    }

    progress(app, "download", 93.5, &format!("Installing Java {major}…"));
    fs::create_dir_all(&java_dir)?;
    let zip_path = java_dir.join("jre.zip");
    fs::write(&zip_path, &bytes)?;
    extract_zip_all(&zip_path, &java_dir)?;
    let _ = fs::remove_file(&zip_path);

    find_java_exe_recursive(&java_dir, exe)
        .ok_or_else(|| anyhow!("Java {major} installed but executable not found in extracted archive"))
}

fn extract_zip_all(zip_path: &PathBuf, dest: &PathBuf) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let out  = dest.join(&name);
        if name.ends_with('/') || name.ends_with('\\') {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() { fs::create_dir_all(p)?; }
        let mut f = fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
    }
    Ok(())
}

/// Download a specific Java major version, emitting `java-progress` events.
/// Used by the setup wizard to download Java 8, 17, and 21 in the background.
pub async fn download_java_major(app: &tauri::AppHandle, major: u32) -> Result<()> {
    let shared_dir = shared_data_dir();
    let exe = if cfg!(windows) { "javaw.exe" } else { "java" };

    let emit = |status: &str, progress: f32, message: &str| {
        let _ = app.emit("java-progress", serde_json::json!({
            "major": major,
            "status": status,
            "progress": progress,
            "message": message,
        }));
    };

    // Only check MLBV's own managed Java — system Java (even Java 25) must not
    // satisfy this check, because v >= r in find_java would skip all downloads.
    let java_dir = shared_dir.join("java").join(format!("jre-{major}"));
    if java_dir.exists() {
        if find_java_exe_recursive(&java_dir, exe).is_some() {
            emit("already", 100.0, "");
            return Ok(());
        }
    }

    emit("downloading", 0.0, "Connecting…");

    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()
        .context("reqwest build")?;

    let os_str   = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch_str = match std::env::consts::ARCH { "x86_64" => "x64", "aarch64" => "aarch64", _ => "x86" };
    // /assets/latest/ is broken (404 for all versions); use /assets/feature_releases/ instead
    let assets_url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{major}/ga?architecture={arch_str}&heap_size=normal&image_type=jre&os={os_str}&vendor=eclipse&page_size=1"
    );

    let assets_json: serde_json::Value = client
        .get(&assets_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot reach Adoptium API")?
        .json().await
        .context("Adoptium assets JSON parse")?;

    let direct_url = assets_json[0]["binaries"][0]["package"]["link"]
        .as_str()
        .ok_or_else(|| anyhow!("Java {major} not found in Adoptium catalog (os={os_str} arch={arch_str})"))?
        .to_string();

    let resp = client.get(&direct_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send().await
        .context("Cannot download Java from CDN")?;
    if !resp.status().is_success() {
        return Err(anyhow!("Java {major} download HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut bytes: Vec<u8> = Vec::new();
    let mut downloaded: u64 = 0;
    let mut last_mb: u64 = 0;
    let mut resp = resp;

    while let Some(chunk) = resp.chunk().await.context("Download interrupted")? {
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let mb = downloaded / 1_048_576;
        if mb > last_mb {
            last_mb = mb;
            let pct = if total > 0 {
                (downloaded as f32 / total as f32) * 90.0
            } else {
                (mb as f32 * 1.5_f32).min(85.0)
            };
            let tot_s = if total > 0 { format!("/{}", total / 1_048_576) } else { String::new() };
            emit("downloading", pct, &format!("Java {major}: {mb}{tot_s} MB"));
        }
    }

    emit("installing", 92.0, &format!("Installing Java {major}…"));
    fs::create_dir_all(&java_dir)?;
    let zip_path = java_dir.join("jre.zip");
    fs::write(&zip_path, &bytes)?;
    extract_zip_all(&zip_path, &java_dir)?;
    let _ = fs::remove_file(&zip_path);

    if find_java_exe_recursive(&java_dir, exe).is_some() {
        emit("done", 100.0, "");
        Ok(())
    } else {
        Err(anyhow!("Java {major} installed but executable not found in extracted archive"))
    }
}

pub fn scan_java_installs() -> Vec<(u32, String)> {
    let exe = if cfg!(windows) { "javaw.exe" } else { "java" };
    let mut found: Vec<(u32, String)> = vec![];

    // mlbv's own downloads
    let shared = shared_data_dir();
    let mlbv_java = shared.join("java");
    if let Ok(entries) = fs::read_dir(&mlbv_java) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
            if let Some(major) = folder_java_major(&entry.file_name().to_string_lossy()) {
                if find_java_exe_recursive(&entry.path(), exe).is_some() {
                    found.push((major, entry.path().to_string_lossy().into_owned()));
                }
            }
        }
    }

    // Common install paths
    #[cfg(windows)]
    {
        let bases = [
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\Microsoft",
            "C:\\Program Files\\Amazon Corretto",
            "C:\\Program Files\\Java",
            "C:\\Program Files (x86)\\Java",
        ];
        for base in &bases {
            if let Ok(rd) = std::fs::read_dir(base) {
                for entry in rd.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(major) = folder_java_major(&name) {
                        let p = entry.path().join("bin").join(exe);
                        if p.exists() && !found.iter().any(|(v, _)| *v == major) {
                            found.push((major, entry.path().to_string_lossy().into_owned()));
                        }
                    }
                }
            }
        }
    }

    found.sort_by_key(|(v, _)| *v);
    found
}
