//! Download steps shared by every loader (libraries, assets) plus the JVM /
//! game argument builder.
use super::*;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

// ─── Shared download steps ───────────────────────────────────────────────────

/// One library file to fetch into the shared cache.
pub(super) struct LibDlJob {
    pub(super) url: String,
    pub(super) path: PathBuf,
    pub(super) size: u64,
    pub(super) sha1: Option<String>,
    /// Human-readable label for error context (`"library com.google.guava:guava:32.0"`).
    pub(super) label: String,
}

/// Download library files concurrently (the same semaphore pattern as the
/// assets). Unlike a missing texture, a missing library breaks the game, so
/// this is fail-fast where assets are best-effort: every job runs (populating
/// the cache for a retry), then the first error aborts the launch. Callers
/// build the classpath afterwards in manifest order — only fetching is
/// parallel, so the command line is identical to the old sequential code.
pub(super) async fn download_libs_parallel(
    ctx: &Ctx<'_>,
    jobs: Vec<LibDlJob>,
    stage: &str,
    pct_start: f32,
    pct_range: f32,
    concurrent: u32,
) -> Result<()> {
    ctx.gate().await?;
    let jobs: Vec<LibDlJob> = jobs.into_iter()
        .filter(|j| !is_valid_file(&j.path, j.size))
        .collect();
    let total = jobs.len();
    if total == 0 { return Ok(()); }
    ctx.progress("download", pct_start, &format!("{stage} (0/{total})…"));
    let sem  = Arc::new(Semaphore::new(concurrent.max(1) as usize));
    let done = Arc::new(AtomicUsize::new(0));
    let mut set = tokio::task::JoinSet::<Result<(), String>>::new();
    for job in jobs {
        if ctx.cancelled() { break; }
        let sem_c    = sem.clone();
        let done_c   = done.clone();
        let client_c = ctx.client.clone();
        let app_c    = ctx.app.clone();
        let ctl_c    = ctx.ctl.clone();
        let inst_c   = ctx.instance.clone();
        let stage_c  = stage.to_string();
        set.spawn(async move {
            // The binding keeps the permit alive for the whole download —
            // without it the limit would not apply (same as the assets).
            let _permit = sem_c.acquire_owned().await.map_err(|e| e.to_string())?;
            if ctl_c.cancel.load(Ordering::Relaxed) { return Ok(()); }
            while ctl_c.pause.load(Ordering::Relaxed) {
                if ctl_c.cancel.load(Ordering::Relaxed) { return Ok(()); }
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            }
            let r = download_file(&client_c, &job.url, &job.path, job.size,
                    job.sha1.as_deref(), Some(&ctl_c.bytes)).await
                .map_err(|e| format!("Downloading {}: {e:#}", job.label));
            let n = done_c.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(8) || n == total {
                let pct = pct_start + (n as f32 / total as f32) * pct_range;
                progress(&app_c, &inst_c, "download", pct, &format!("{stage_c} ({n}/{total})…"));
            }
            r
        });
    }
    let mut first_err: Option<String> = None;
    while let Some(r) = set.join_next().await {
        match r {
            Ok(Ok(())) => {}
            Ok(Err(e)) => { if first_err.is_none() { first_err = Some(e); } }
            Err(e) => {
                if first_err.is_none() { first_err = Some(format!("Download task failed: {e}")); }
            }
        }
    }
    if ctx.cancelled() { return Err(anyhow!("Download cancelled")); }
    if let Some(e) = first_err { return Err(anyhow!("{e}")); }
    Ok(())
}

/// Download the vanilla libraries into the shared cache and extract the
/// platform natives into the instance. Returns the library classpath.
pub(super) async fn download_vanilla_libraries(
    ctx: &Ctx<'_>,
    ver: &VersionJson,
    libs_dir: &Path,
    natives_dir: &Path,
    concurrent: u32,
) -> Result<Vec<String>> {
    // Plan first: every file to fetch, plus the ordered classpath entries.
    let mut jobs: Vec<LibDlJob> = Vec::new();
    let mut artifacts: Vec<PathBuf> = Vec::new();
    let mut natives: Vec<PathBuf> = Vec::new();
    for lib in &ver.libraries {
        if !lib_allowed(lib) { continue; }
        let Some(dl) = &lib.downloads else { continue };

        if let Some(art) = &dl.artifact {
            let path = libs_dir.join(&art.path);
            jobs.push(LibDlJob {
                url: art.url.clone(),
                path: path.clone(),
                size: art.size,
                sha1: Some(art.sha1.clone()),
                label: format!("library {}", lib.name),
            });
            artifacts.push(path);
        }

        if let Some(natives_map) = &lib.natives {
            if let Some(classifier) = natives_map.get(os_classifier_key()) {
                let classifier = classifier.replace("${arch}", arch_bits());
                if let Some(nat) = dl.classifiers.as_ref().and_then(|c| c.get(&classifier)) {
                    let path = libs_dir.join(&nat.path);
                    jobs.push(LibDlJob {
                        url: nat.url.clone(),
                        path: path.clone(),
                        size: nat.size,
                        sha1: Some(nat.sha1.clone()),
                        label: format!("native {}", lib.name),
                    });
                    natives.push(path);
                }
            }
        }
    }

    download_libs_parallel(ctx, jobs, "Libraries",
        PCT_LIBS, PCT_LIBS_END - PCT_LIBS, concurrent).await?;

    // Classpath in manifest order, then the (fast, local) natives extraction.
    let mut classpath: Vec<String> = Vec::with_capacity(artifacts.len());
    for path in &artifacts {
        classpath.push(path.to_string_lossy().into_owned());
    }
    for path in &natives {
        ctx.gate().await?;
        extract_natives(path, natives_dir)
            .with_context(|| format!("Extracting natives {}", path.display()))?;
    }
    Ok(classpath)
}

/// Asset index + parallel asset objects, both in the shared cache.
pub(super) async fn download_assets(ctx: &Ctx<'_>, ver: &VersionJson, concurrent: u32) -> Result<()> {
    ctx.progress("download", PCT_INDEX, "Downloading asset index…");
    let idx_dir = ctx.shared.join("assets").join("indexes");
    fs::create_dir_all(&idx_dir)?;
    let idx_path = idx_dir.join(format!("{}.json", ver.asset_index.id));
    if !idx_path.exists() {
        let text = ctx.client.get(&ver.asset_index.url).send().await?.text().await?;
        fs::write(&idx_path, &text)?;
    }

    let idx: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;
    let objs_dir = ctx.shared.join("assets").join("objects");
    let total = idx.objects.len();
    ctx.progress("download", PCT_ASSETS, &format!("Assets (0/{total})…"));
    download_assets_parallel(ctx, &idx.objects, &objs_dir, concurrent,
        PCT_ASSETS, PCT_ASSETS_END - PCT_ASSETS, idx.map_to_resources).await?;
    if ctx.cancelled() { return Err(anyhow!("Download cancelled")); }
    ctx.progress("download", PCT_ASSETS_END, &format!("Assets ({total}/{total})…"));
    Ok(())
}

// ─── Command line ────────────────────────────────────────────────────────────

/// Push JSON-encoded arguments (loader profile / Forge overlay) through the
/// variable replacement, honouring their `rules`.
pub(super) fn push_json_args(
    args: &[serde_json::Value],
    replace: &impl Fn(&str) -> String,
    out: &mut Vec<String>,
) {
    for v in args {
        match v {
            serde_json::Value::String(s) => out.push(replace(s)),
            serde_json::Value::Object(_) => {
                let allowed = v["rules"].as_array().is_none_or(|rs| rs.iter().any(|r| {
                    r["action"].as_str() == Some("allow") && {
                        let os_name = r["os"]["name"].as_str().unwrap_or("");
                        os_name.is_empty()
                            || (cfg!(windows) && os_name == "windows")
                            || (cfg!(target_os = "macos") && os_name == "osx")
                            || (cfg!(target_os = "linux") && os_name == "linux")
                    }
                }));
                if !allowed { continue; }
                match &v["value"] {
                    serde_json::Value::String(s) => out.push(replace(s)),
                    serde_json::Value::Array(arr) => {
                        out.extend(arr.iter().filter_map(|x| x.as_str()).map(replace));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// User-tunable JVM settings, grouped so `build_launch_args` stays readable.
pub(super) struct JvmOptions<'a> {
    pub(super) max_ram_mb: u32,
    pub(super) min_ram_mb: u32,
    /// Raw extra flags from instance settings, whitespace-separated.
    pub(super) extra_args: &'a str,
}

/// Build the full `java …` argument list. Order matters: loader JVM args first
/// (they may add module paths), then the vanilla ones, then memory, main class,
/// vanilla game args and finally the overlay game args — the same order an
/// `inheritsFrom` overlay implies.
pub(super) fn build_launch_args(
    ver: &VersionJson,
    plan: &LaunchPlan,
    main_class: &str,
    replace: &impl Fn(&str) -> String,
    natives_dir: &Path,
    classpath_str: &str,
    jvm: &JvmOptions<'_>,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    push_json_args(&plan.overlay_jvm, replace, &mut args);

    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.jvm { resolve_arg(arg, replace, &mut args); }
    } else {
        // Pre-1.13 version JSONs carry no JVM argument list.
        args.push(format!("-Djava.library.path={}", natives_dir.display()));
        args.push("-Dminecraft.launcher.brand=MLBV".to_string());
        args.push("-Dminecraft.launcher.version=1.0".to_string());
        args.push("-cp".to_string());
        args.push(classpath_str.to_string());
    }
    // Clamp insanity: the JVM refuses to start with -Xmx0m, and a minimum
    // above the maximum is equally fatal.
    let max_ram = jvm.max_ram_mb.max(512);
    let min_ram = jvm.min_ram_mb.clamp(256, max_ram);
    args.push(format!("-Xmx{max_ram}m"));
    args.push(format!("-Xms{min_ram}m"));
    // User JVM args go after the launcher defaults so they win on conflict
    // (a custom -Xmx in Settings → Java overrides the slider, by design).
    for part in jvm.extra_args.split_whitespace() {
        args.push(part.to_string());
    }
    args.push(main_class.to_string());

    if let Some(new_args) = &ver.arguments {
        for arg in &new_args.game { resolve_arg(arg, replace, &mut args); }
    } else if let Some(old) = &ver.minecraft_arguments {
        for part in old.split_whitespace() { args.push(replace(part)); }
    }

    if !plan.overlay_game.is_empty() {
        push_json_args(&plan.overlay_game, replace, &mut args);
    } else if let Some(old) = &plan.overlay_minecraft_arguments {
        for part in old.split_whitespace() { args.push(replace(part)); }
    }

    args
}

// ─── Parallel asset download ─────────────────────────────────────────────────

pub(super) async fn download_assets_parallel(
    ctx: &Ctx<'_>,
    objects: &HashMap<String, AssetObj>,
    objs_dir: &Path,
    concurrent: u32,
    pct_start: f32,
    pct_range: f32,
    map_to_resources: bool,
) -> Result<()> {
    let app = ctx.app;
    let client = ctx.client;
    let ctl = ctx.ctl.clone();
    let total = objects.len();
    if total == 0 {
        if map_to_resources { map_legacy_assets(objects, objs_dir)?; }
        return Ok(());
    }
    let sem  = Arc::new(Semaphore::new(concurrent.max(1) as usize));
    let done = Arc::new(AtomicUsize::new(0));
    let mut set = tokio::task::JoinSet::<()>::new();
    for obj in objects.values() {
        if ctl.cancel.load(Ordering::Relaxed) { break; }
        let prefix   = obj.hash[..2].to_string();
        let hash     = obj.hash.clone();
        let size     = obj.size;
        let obj_dir  = objs_dir.join(&prefix);
        let obj_path = obj_dir.join(&hash);
        let sem_c    = sem.clone();
        let done_c   = done.clone();
        let client_c = client.clone();
        let app_c    = app.clone();
        let ctl_c    = ctl.clone();
        let inst_c   = ctx.instance.clone();
        set.spawn(async move {
            let _permit = sem_c.acquire_owned().await.unwrap();
            if ctl_c.cancel.load(Ordering::Relaxed) { return; }
            // Pause loop
            while ctl_c.pause.load(Ordering::Relaxed) {
                if ctl_c.cancel.load(Ordering::Relaxed) { return; }
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            }
            if !is_valid_file(&obj_path, size) {
                let _ = fs::create_dir_all(&obj_dir);
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    prefix, hash
                );
                // Best-effort per asset: a single failed object means one
                // missing texture, not a broken launch.
                let _ = download_file(&client_c, &url, &obj_path, size, Some(&hash), Some(&ctl_c.bytes)).await;
            }
            let n = done_c.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(50) || n == total {
                let pct = pct_start + (n as f32 / total as f32) * pct_range;
                progress(&app_c, &inst_c, "download", pct, &format!("Assets ({n}/{total})…"));
            }
        });
    }
    while set.join_next().await.is_some() {}
    if map_to_resources {
        map_legacy_assets(objects, objs_dir).context("Mapping legacy assets")?;
    }
    Ok(())
}
