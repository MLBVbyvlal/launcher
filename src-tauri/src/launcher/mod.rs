//! The launch pipeline. `launch` does the work shared by every loader
//! (manifest, version JSON, client JAR, libraries, natives, assets, Java,
//! arguments, spawn); the loader-specific overlay lives in `loaders`.
//!
//! Submodules: `loaders` (Fabric/Quilt/Forge/NeoForge/LiquidBounce steps),
//! `java` (runtime discovery + Adoptium provisioning), `process` (spawn,
//! exit watch, JVM output buffer), `util` (verified downloads, ZIP, paths,
//! rule evaluation).

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};

mod java;
mod loaders;
mod process;
mod state;
mod steps;
mod types;
mod util;

pub use java::{download_java_major, find_java_exe_recursive, folder_java_major, scan_java_installs};
use java::ensure_java;
pub use util::mc_dir;
use loaders::{prepare_loader, prepare_loader_stage, LbManifest};
use process::spawn_game;
pub use state::{DlControl, GameState};
use state::{AbortOnDrop, DownloadGuard};
use steps::*;
pub use types::JavaVersionReq;
use types::*;
use util::*;
// ─── Progress event ───────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct LaunchProgress {
    pub instance: String,
    pub stage: String,
    pub progress: f32,
    pub message: String,
}

/// Progress is always attributed to an instance: several launches may be in
/// flight and the UI shows each one on its own card.
fn progress(app: &tauri::AppHandle, instance: &str, stage: &str, pct: f32, msg: &str) {
    let _ = app.emit("launch-progress", LaunchProgress {
        instance: instance.into(),
        stage: stage.into(),
        progress: pct,
        message: msg.into(),
    });
}

// ─── Instance name validation ─────────────────────────────────────────────────

/// Whitelist-check an instance name before it is used as a path segment.
/// Instance names come from the frontend (localStorage) and reach
/// `join()` + `remove_dir_all` in several commands — an unvalidated name
/// could escape the instances dir (`../../`) or name a Windows device.
pub fn valid_instance_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(anyhow!("Invalid instance name (length 1–64)"));
    }
    if name.contains('/') || name.contains('\\') || name.contains(':') || name.contains('\0') {
        return Err(anyhow!("Invalid instance name (forbidden characters)"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.' | '+')) {
        return Err(anyhow!("Invalid instance name (allowed: letters, digits, space, _ - . +)"));
    }
    if name.starts_with('.') || name == "." || name == ".." || name.ends_with('.') || name.ends_with(' ') {
        return Err(anyhow!("Invalid instance name (bad start/end character)"));
    }
    // Windows reserves device names with any extension ("CON.txt" is still
    // CON), so the stem before the first dot is what matters.
    let base = name.trim_end_matches(['.', ' ']).to_ascii_uppercase();
    let stem = base.split('.').next().unwrap_or(base.as_str());
    const RESERVED: [&str; 17] = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4",
    ];
    if RESERVED.contains(&stem) {
        return Err(anyhow!("Invalid instance name (Windows reserved name)"));
    }
    Ok(())
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

// ─── Launch pipeline — shared by every loader ────────────────────────────────
//
// Vanilla, Fabric, Quilt, Forge, NeoForge and LiquidBounce all do the same
// thing: fetch the version manifest, download the client JAR, libraries,
// natives and assets, find Java, build the argument list, spawn the process.
// Only the "overlay" on top of vanilla differs, so that is the only part that
// is per-loader (`prepare_loader` / `prepare_loader_stage`). Everything else
// lives here once — a fix in this file now reaches all six loaders.

/// Mod loader of an instance. The frontend sends this as the `loader` argument
/// of the single `launch_game` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    Vanilla,
    Liquidbounce,
    Fabric,
    Quilt,
    Forge,
    Neoforge,
}

impl Loader {
    /// Label used in progress messages ("Minecraft (Fabric)") and in the final
    /// "… launched!" message.
    fn label(self) -> &'static str {
        match self {
            Loader::Vanilla      => "Minecraft",
            Loader::Liquidbounce => "LiquidBounce",
            Loader::Fabric       => "Minecraft (Fabric)",
            Loader::Quilt        => "Minecraft (Quilt)",
            Loader::Forge        => "Minecraft (Forge)",
            Loader::Neoforge     => "Minecraft (NeoForge)",
        }
    }

    /// Short name for progress lines and library messages ("Forge").
    fn short_label(self) -> &'static str {
        match self {
            Loader::Vanilla      => "Vanilla",
            Loader::Liquidbounce => "LiquidBounce",
            Loader::Fabric       => "Fabric",
            Loader::Quilt        => "Quilt",
            Loader::Forge        => "Forge",
            Loader::Neoforge     => "NeoForge",
        }
    }
}

/// Everything the launch pipeline needs. `loader_version` is the Fabric/Quilt
/// loader version or the full Forge/NeoForge version (`"1.20.1-47.3.11"` /
/// `"21.1.172"`); `lb_build_id` is only meaningful for LiquidBounce. An empty
/// version means "resolve the newest stable automatically".
pub struct LaunchRequest {
    pub loader: Loader,
    pub mc_version: String,
    pub loader_version: String,
    pub lb_build_id: u32,
    pub instance_name: String,
    pub username: String,
    pub uuid: String,
    pub offline: bool,
    pub access_token: String,
    pub concurrent_downloads: u32,
    pub max_ram_mb: u32,
    /// Custom Java executable from Settings (empty = auto-detect).
    pub java_path: String,
    /// Extra JVM arguments from Settings, whitespace-separated (may be empty).
    pub jvm_args: String,
    /// `-Xms` value in MB (the old default was a hardcoded 256).
    pub min_ram_mb: u32,
}

/// Shared context for the launch steps: paths, HTTP client and the
/// download-control flags. Steps borrow it, so none can outlive them.
struct Ctx<'a> {
    pub(super) app:      &'a tauri::AppHandle,
    pub(super) client:   &'a reqwest::Client,
    pub(super) instance: String,
    pub(super) ctl:      Arc<DlControl>,
    pub(super) shared:   PathBuf,
    pub(super) game_dir: PathBuf,
}

impl Ctx<'_> {
    fn cancelled(&self) -> bool {
        self.ctl.cancel.load(Ordering::Relaxed)
    }

    fn progress(&self, stage: &str, pct: f32, msg: &str) {
        progress(self.app, &self.instance, stage, pct, msg);
    }

    /// Sleep while the user has paused the download; fail if it was cancelled.
    /// Called between every download so pause/cancel work in all loaders.
    async fn gate(&self) -> Result<()> {
        while self.ctl.pause.load(Ordering::Relaxed) {
            if self.cancelled() { return Err(anyhow!("Download cancelled")); }
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }
        if self.cancelled() { return Err(anyhow!("Download cancelled")); }
        Ok(())
    }
}

// Progress schedule. One set of numbers for every loader, so the bar means the
// same thing whichever pipeline is running (the old copies each had their own).
const PCT_MANIFEST: f32 = 4.0;
const PCT_VERSION:  f32 = 8.0;
const PCT_JAR:      f32 = 12.0;
const PCT_LIBS:     f32 = 15.0;
const PCT_LIBS_END: f32 = 45.0;
const PCT_INDEX:    f32 = 48.0;
const PCT_ASSETS:   f32 = 50.0;
const PCT_ASSETS_END: f32 = 78.0;
const PCT_LOADER:   f32 = 80.0;
const PCT_LOADER_END: f32 = 88.0;
const PCT_MODS:     f32 = 88.0;
const PCT_JAVA:     f32 = 92.0;
const PCT_START:    f32 = 96.0;

const MODRINTH_UA: &str = "MLBV/1.0 (github.com/MLBVbyvlalikoffc/launcher)";

/// What the loader needs before the vanilla download can start.
enum LoaderPrep {
    /// Vanilla: nothing to resolve.
    Nothing,
    /// Fabric / Quilt: loader version resolved, profile fetched later.
    MetaProfile { loader_ver: String },
    /// Forge / NeoForge: installer already ran, this is the overlay version JSON.
    Overlay { json: serde_json::Value, ver_name: String },
    /// LiquidBounce: launch manifest (fabric loader version + mod list).
    Lb { manifest: LbManifest },
}

impl LoaderPrep {
    /// Forge/NeoForge overlay JSONs declare the vanilla version they inherit
    /// from, which can differ from the one the user picked.
    fn base_mc_version(&self, fallback: &str) -> String {
        match self {
            LoaderPrep::Overlay { json, .. } =>
                json["inheritsFrom"].as_str().unwrap_or(fallback).to_string(),
            _ => fallback.to_string(),
        }
    }
}

/// The loader-specific part of the command line.
struct LaunchPlan {
    /// `${version_name}` — shown in the F3 screen and the crash report.
    pub(super) version_name: String,
    /// Main class override; `None` means the vanilla one from the version JSON.
    pub(super) main_class: Option<String>,
    /// Loader libraries, appended to the vanilla classpath before the JAR.
    pub(super) classpath: Vec<String>,
    /// Overlay `arguments.jvm` / `arguments.game` (profile or Forge JSON).
    pub(super) overlay_jvm: Vec<serde_json::Value>,
    pub(super) overlay_game: Vec<serde_json::Value>,
    /// Pre-1.13 style overlay game arguments.
    pub(super) overlay_minecraft_arguments: Option<String>,
    /// Extra substitution variables (Forge needs `${library_directory}`).
    pub(super) extra_vars: Vec<(String, String)>,
}

pub async fn launch(app: tauri::AppHandle, req: LaunchRequest) -> Result<()> {
    let shared      = shared_data_dir();
    let game_dir    = instances_dir().join(&req.instance_name);
    let mods_dir    = game_dir.join("mods");
    // Natives are extracted per instance (like PrismLauncher): two instances of
    // the same version running side by side must not race on a shared dir.
    let natives_dir = game_dir.join("natives");

    for d in ["saves", "screenshots", "resourcepacks", "texturepacks", "shaderpacks",
              "logs", "crash-reports", "config", "datapacks", "mods"] {
        let _ = fs::create_dir_all(game_dir.join(d));
    }
    fs::create_dir_all(&shared)?;
    fs::create_dir_all(&mods_dir)?;
    fs::create_dir_all(&natives_dir)?;

    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build()?;

    // Download controls belong to this launch only; other instances keep
    // their own pause/cancel state and speed counter.
    let state = app.state::<GameState>();
    let ctl = state.begin_download(&req.instance_name);
    let _dl_guard = DownloadGuard { state: &state, instance: req.instance_name.clone() };

    // Speed monitor: emits "download-speed" every second while running
    let _speed_guard = {
        let c = ctl.clone();
        let a = app.clone();
        let inst = req.instance_name.clone();
        let h = tokio::spawn(async move {
            let mut last = 0u64;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let cur = c.bytes.load(Ordering::Relaxed);
                let bps = cur.saturating_sub(last);
                last = cur;
                let _ = a.emit("download-speed", serde_json::json!({ "instance": inst, "bps": bps }));
            }
        });
        AbortOnDrop(h)
    };

    let ctx = Ctx {
        app: &app,
        client: &client,
        instance: req.instance_name.clone(),
        ctl,
        shared: shared.clone(),
        game_dir: game_dir.clone(),
    };

    // ── 1. Loader preparation: installer / loader version / LB manifest ──
    let prep = prepare_loader(&ctx, &req).await?;
    let mc_ver = prep.base_mc_version(&req.mc_version);

    // Two launches of the same version share every cache file below; the
    // second one waits here (visibly) until the first has finished writing.
    let version_lock = state.version_lock(&mc_ver);
    let _version_permit = match version_lock.try_lock() {
        Ok(p) => p,
        Err(_) => {
            ctx.progress("wait", PCT_MANIFEST, &format!("Waiting for another download of {mc_ver}…"));
            loop {
                ctx.gate().await?;
                match tokio::time::timeout(tokio::time::Duration::from_millis(500), version_lock.lock()).await {
                    Ok(p) => break p,
                    Err(_) => continue,
                }
            }
        }
    };

    // ── 2. Vanilla base: manifest, version JSON, client JAR ──
    ctx.progress("fetch", PCT_MANIFEST, "Fetching version manifest…");
    let manifest: VersionManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await
        .context("Cannot reach Mojang servers")?
        .json().await?;
    let entry = manifest.versions.iter()
        .find(|v| v.id == mc_ver)
        .ok_or_else(|| anyhow!("Version '{}' not found in manifest", mc_ver))?;

    ctx.progress("fetch", PCT_VERSION, "Fetching version info…");
    let ver_text = client.get(&entry.url).send().await?.text().await?;
    let ver: VersionJson = serde_json::from_str(&ver_text)?;

    // Version metadata and the client JAR live in the shared cache only — one
    // copy per Minecraft version, reused by every instance (see README).
    let ver_dir = shared.join("versions").join(&mc_ver);
    fs::create_dir_all(&ver_dir)?;
    let ver_json_path = ver_dir.join(format!("{mc_ver}.json"));
    if !ver_json_path.exists() { fs::write(&ver_json_path, &ver_text)?; }

    let jar_path = ver_dir.join(format!("{mc_ver}.jar"));
    if !is_valid_file(&jar_path, ver.downloads.client.size) {
        ctx.progress("download", PCT_JAR, "Downloading Minecraft client…");
        download_file(&client, &ver.downloads.client.url, &jar_path,
            ver.downloads.client.size, Some(&ver.downloads.client.sha1), Some(&ctx.ctl.bytes)).await
            .context("Failed to download Minecraft client")?;
    }

    // ── 3. Vanilla libraries and natives ──
    let libs_dir = shared.join("libraries");
    let mut classpath = download_vanilla_libraries(&ctx, &ver, &libs_dir, &natives_dir, req.concurrent_downloads).await?;

    // ── 4. Assets ──
    download_assets(&ctx, &ver, req.concurrent_downloads).await?;

    // ── 5. Loader overlay: profile/JSON libraries, mods ──
    let plan = prepare_loader_stage(&ctx, &req, &prep, &mc_ver, &libs_dir).await?;
    classpath.extend(plan.classpath.iter().cloned());
    // The client JAR must be last on the classpath.
    classpath.push(jar_path.to_string_lossy().into_owned());

    // ── 6. Java runtime ──
    ctx.progress("launch", PCT_JAVA, "Finding Java runtime…");
    let java_override = if req.java_path.trim().is_empty() { None } else { Some(req.java_path.trim()) };
    let java = ensure_java(&app, &client, &shared, ver.java_version.as_ref(), java_override, Some(&ctx)).await
        .context("Failed to obtain Java runtime")?;

    // ── 7. Command line ──
    ctx.progress("launch", PCT_START, &format!("Starting {}…", req.loader.label()));
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath_str = classpath.join(sep);
    let token     = if req.offline { "0".to_string() } else { req.access_token.clone() };
    let user_type = if req.offline { "offline" } else { "msa" };
    let main_class = plan.main_class.clone().unwrap_or_else(|| ver.main_class.clone());

    let mut vars: HashMap<&str, String> = HashMap::from([
        ("${auth_player_name}",    req.username.clone()),
        ("${version_name}",        plan.version_name.clone()),
        ("${game_directory}",      game_dir.to_string_lossy().into_owned()),
        ("${assets_root}",         shared.join("assets").to_string_lossy().into_owned()),
        ("${assets_index_name}",   ver.asset_index.id.clone()),
        ("${auth_uuid}",           req.uuid.clone()),
        ("${auth_access_token}",   token),
        ("${user_type}",           user_type.to_string()),
        ("${version_type}",        "release".to_string()),
        ("${user_properties}",     "{}".to_string()),
        ("${natives_directory}",   natives_dir.to_string_lossy().into_owned()),
        ("${launcher_name}",       "MLBV".to_string()),
        ("${launcher_version}",    "1.0".to_string()),
        ("${classpath}",           classpath_str.clone()),
        ("${classpath_separator}", sep.to_string()),
        ("${resolution_width}",    "854".to_string()),
        ("${resolution_height}",   "480".to_string()),
    ]);
    for (k, v) in &plan.extra_vars {
        vars.insert(k.as_str(), v.clone());
    }

    let replace = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars { out = out.replace(k, v); }
        out
    };

    let cmd_args = build_launch_args(&ver, &plan, &main_class, &replace,
        &natives_dir, &classpath_str, req.max_ram_mb, req.min_ram_mb, &req.jvm_args);

    // ── 8. Spawn and track ──
    spawn_game(&app, &java, &cmd_args, &game_dir, &req.instance_name)?;
    ctx.progress("launch", 100.0, &format!("{} launched!", req.loader.label()));
    Ok(())
}
