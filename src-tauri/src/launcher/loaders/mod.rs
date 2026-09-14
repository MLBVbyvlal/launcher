//! Loader overlay: what each loader adds before and after the vanilla
//! download. Anything here reaches one loader only — see AGENTS.md §3.
use super::*;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

mod lb;
mod overlay;

use lb::*;
use overlay::*;
// ─── Loader API types ────────────────────────────────────────────────────────

/// LiquidBounce launch manifest (`/api/v1/version/launch/{build}`).
#[derive(Deserialize)]
pub(super) struct LbManifest {
    pub(super) build: LbManifestBuild,
    #[serde(default)]
    pub(super) mods: Vec<LbMod>,
    #[serde(default)]
    pub(super) repositories: HashMap<String, String>,
}

#[derive(Deserialize)]
pub(super) struct LbManifestBuild {
    pub(super) fabric_loader_version: String,
}

#[derive(Deserialize)]
pub(super) struct LbMod {
    pub(super) required: bool,
    pub(super) name: String,
    pub(super) source: serde_json::Value,
}

/// Fabric and Quilt both serve a version-JSON overlay at
/// `/versions/loader/{mc}/{loader}/profile/json`, so one type covers both.
#[derive(Deserialize)]
pub(super) struct FabricProfile {
    #[serde(rename = "mainClass")]
    pub(super) main_class: String,
    #[serde(default)]
    pub(super) libraries: Vec<FabricLibrary>,
    #[serde(default)]
    pub(super) arguments: Option<FabricArguments>,
}

#[derive(Deserialize)]
pub(super) struct FabricLibrary {
    pub(super) name: String,
    pub(super) url: String,
}

#[derive(Deserialize)]
pub(super) struct FabricArguments {
    #[serde(default)]
    pub(super) jvm: Vec<serde_json::Value>,
}
// ─── Loader preparation (before the vanilla download) ────────────────────────

pub(super) async fn prepare_loader(ctx: &Ctx<'_>, req: &LaunchRequest) -> Result<LoaderPrep> {
    match req.loader {
        Loader::Vanilla => Ok(LoaderPrep::Nothing),

        Loader::Fabric => Ok(LoaderPrep::MetaProfile {
            loader_ver: resolve_meta_loader_version(ctx, "https://meta.fabricmc.net/v2",
                "FabricMC", "Fabric Loader", &req.mc_version, &req.loader_version).await?,
        }),

        Loader::Quilt => Ok(LoaderPrep::MetaProfile {
            loader_ver: resolve_meta_loader_version(ctx, "https://meta.quiltmc.org/v3",
                "QuiltMC", "Quilt Loader", &req.mc_version, &req.loader_version).await?,
        }),

        Loader::Forge => {
            // The installer names the version "{mc}-forge-{forgeOnly}".
            let forge_only = req.loader_version
                .strip_prefix(&format!("{}-", req.mc_version))
                .unwrap_or(&req.loader_version);
            let (json, ver_name) = run_loader_installer(ctx, &InstallerSpec {
                kind: "Forge",
                installer_url: format!(
                    "https://maven.minecraftforge.net/net/minecraftforge/forge/{v}/forge-{v}-installer.jar",
                    v = req.loader_version
                ),
                installer_dir: "forge-installers",
                installer_name: format!("forge-{}-installer.jar", req.loader_version),
                ver_name: format!("{}-forge-{}", req.mc_version, forge_only),
                dir_hint: "forge",
                dir_mc_filter: Some(req.mc_version.clone()),
                java_major: 17,
            }).await?;
            Ok(LoaderPrep::Overlay { json, ver_name })
        }

        Loader::Neoforge => {
            let (json, ver_name) = run_loader_installer(ctx, &InstallerSpec {
                kind: "NeoForge",
                installer_url: format!(
                    "https://maven.neoforged.net/releases/net/neoforged/neoforge/{v}/neoforge-{v}-installer.jar",
                    v = req.loader_version
                ),
                installer_dir: "neoforge-installers",
                installer_name: format!("neoforge-{}-installer.jar", req.loader_version),
                ver_name: format!("neoforge-{}", req.loader_version),
                dir_hint: "neoforge",
                dir_mc_filter: None,
                java_major: 21,
            }).await?;
            Ok(LoaderPrep::Overlay { json, ver_name })
        }

        Loader::Liquidbounce => {
            ctx.progress("fetch", 5.0, "Fetching LiquidBounce manifest…");
            let text = ctx.client
                .get(format!("https://api.liquidbounce.net/api/v1/version/launch/{}", req.lb_build_id))
                .send().await
                .context("Cannot reach LiquidBounce API")?
                .text().await?;
            let manifest: LbManifest = serde_json::from_str(&text)
                .map_err(|e| anyhow!("Manifest parse: {e}"))?;
            Ok(LoaderPrep::Lb { manifest })
        }
    }
}

/// Resolve a Fabric/Quilt loader version: keep the pinned one, otherwise pick
/// the newest stable entry (Fabric marks stability explicitly; Quilt's list is
/// already newest-first, so the fallback to `first()` covers it).
pub(super) async fn resolve_meta_loader_version(
    ctx: &Ctx<'_>,
    api_base: &str,
    api_name: &str,
    label: &str,
    mc_version: &str,
    pinned: &str,
) -> Result<String> {
    if !pinned.is_empty() {
        ctx.progress("fetch", 5.0, &format!("Using {label} {pinned}…"));
        return Ok(pinned.to_string());
    }
    ctx.progress("fetch", 5.0, &format!("Fetching {label} version…"));
    let list: serde_json::Value = ctx.client
        .get(format!("{api_base}/versions/loader/{mc_version}"))
        .send().await
        .with_context(|| format!("Cannot reach {api_name} API"))?
        .json().await?;
    list.as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|e| e["loader"]["stable"].as_bool().unwrap_or(false))
                .or_else(|| arr.first())
        })
        .and_then(|e| e["loader"]["version"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("No {label} found for MC {mc_version}"))
}

pub(super) struct InstallerSpec {
    pub(super) kind: &'static str,
    pub(super) installer_url: String,
    pub(super) installer_dir: &'static str,
    pub(super) installer_name: String,
    /// Expected `shared/versions/<ver_name>/` directory.
    pub(super) ver_name: String,
    /// Substring used to locate the directory if the installer named it differently.
    pub(super) dir_hint: &'static str,
    pub(super) dir_mc_filter: Option<String>,
    /// Java the installer itself needs (Forge: 17, NeoForge: 21).
    pub(super) java_major: u32,
}

/// Run the Forge/NeoForge installer unless the overlay version JSON is already
/// cached, then return the parsed overlay JSON.
pub(super) async fn run_loader_installer(
    ctx: &Ctx<'_>,
    spec: &InstallerSpec,
) -> Result<(serde_json::Value, String)> {
    let ver_dir   = ctx.shared.join("versions").join(&spec.ver_name);
    let json_path = ver_dir.join(format!("{}.json", spec.ver_name));
    let libs_dir  = ctx.shared.join("libraries");

    // A cached overlay counts only when its libraries are complete — an
    // interrupted install or deleted files must re-run the installer, not
    // launch with a partial classpath.
    let cache_complete = fs::read_to_string(&json_path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .is_some_and(|j| overlay_cache_complete(&j, &libs_dir));

    if !cache_complete {
        let installer_dir = ctx.shared.join(spec.installer_dir);
        fs::create_dir_all(&installer_dir)?;
        let installer_path = installer_dir.join(&spec.installer_name);

        if !installer_path.exists() {
            ctx.progress("download", 10.0, &format!("Downloading {} installer…", spec.kind));
            download_file(ctx.client, &spec.installer_url, &installer_path, 0, None, Some(&ctx.ctl.bytes)).await
                .with_context(|| format!("Failed to download {} installer", spec.kind))?;
        }

        let java_req = JavaVersionReq {
            component: "java-runtime-gamma".to_string(),
            major_version: spec.java_major,
        };
        // The installer needs its own fixed major version, so the user's game
        // Java override deliberately does not apply here (None).
        let java = ensure_java(ctx.app, ctx.client, &ctx.shared, Some(&java_req), None, Some(ctx)).await
            .with_context(|| format!("Failed to find Java for {} installer", spec.kind))?;

        ctx.progress("install", 30.0, &format!("Installing {} (this may take a minute)…", spec.kind));

        // Both installers refuse to run without a launcher_profiles.json in the
        // target directory; they never read it, they only check for it.
        let profiles_path = ctx.shared.join("launcher_profiles.json");
        if !profiles_path.exists() {
            let _ = fs::write(&profiles_path,
                r#"{"profiles":{},"selectedProfile":"(Default)","authenticationDatabase":{},"clientToken":""}"#);
        }

        let java_c      = java.clone();
        let installer_c = installer_path.clone();
        let shared_c    = ctx.shared.clone();
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&java_c)
                // No -Djava.awt.headless — the installer may need AWT to start up
                .arg("-jar")
                .arg(&installer_c)
                .arg("--installClient")
                .arg(&shared_c)
                .current_dir(&shared_c)
                .output()
        }).await?
          .map_err(|e| anyhow!("Failed to spawn {} installer: {e}", spec.kind))?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}\n{stderr}");
            let tail: String = combined.lines().rev().take(40)
                .collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
            return Err(anyhow!(
                "{} installer failed (exit {:?}):\n{}", spec.kind, output.status.code(), tail.trim()
            ));
        }
    }

    if !json_path.exists() {
        // The installer occasionally picks another directory name; report what
        // it created instead of a bare "not found".
        let hint   = spec.dir_hint.to_ascii_lowercase();
        let mc_fit = spec.dir_mc_filter.clone();
        let found = fs::read_dir(ctx.shared.join("versions"))
            .ok()
            .and_then(|rd| rd.flatten().find(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.contains(&hint) && mc_fit.as_deref().map_or(true, |mc| n.contains(mc))
            }))
            .map(|e| e.file_name().to_string_lossy().into_owned());
        if let Some(actual_name) = found {
            return Err(anyhow!(
                "{} installed to '{}' but expected '{}'. Check shared/versions/ manually.",
                spec.kind, actual_name, spec.ver_name
            ));
        }
        return Err(anyhow!("{} version JSON not found after install: {:?}", spec.kind, json_path));
    }

    let text = fs::read_to_string(&json_path)?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow!("{} version JSON parse error: {e}", spec.kind))?;
    Ok((json, spec.ver_name.clone()))
}

// ─── Loader overlay stage (after the vanilla download) ───────────────────────

pub(super) async fn prepare_loader_stage(
    ctx: &Ctx<'_>,
    req: &LaunchRequest,
    prep: &LoaderPrep,
    mc_ver: &str,
    libs_dir: &Path,
) -> Result<LaunchPlan> {
    match prep {
        LoaderPrep::Nothing => Ok(LaunchPlan {
            version_name: mc_ver.to_string(),
            main_class: None,
            classpath: Vec::new(),
            overlay_jvm: Vec::new(),
            overlay_game: Vec::new(),
            overlay_minecraft_arguments: None,
            extra_vars: Vec::new(),
        }),

        LoaderPrep::MetaProfile { loader_ver } => {
            let (api_base, label, default_base, extra_mods) = match req.loader {
                Loader::Fabric => ("https://meta.fabricmc.net/v2", "Fabric", None, true),
                Loader::Quilt => ("https://meta.quiltmc.org/v3", "Quilt",
                    Some("https://maven.quiltmc.org/repository/release/"), false),
                _ => return Err(anyhow!("Internal: loader {:?} has no meta profile", req.loader)),
            };

            let profile = fetch_meta_profile(ctx, api_base, mc_ver, loader_ver, label).await?;
            let classpath = download_profile_libraries(ctx, &profile, default_base, label, req.concurrent_downloads).await?;
            if extra_mods { download_fabric_api(ctx, mc_ver).await?; }

            Ok(LaunchPlan {
                version_name: format!("{}-loader-{loader_ver}-{mc_ver}", label.to_lowercase()),
                main_class: Some(profile.main_class.clone()),
                classpath,
                overlay_jvm: profile.arguments.as_ref().map(|a| a.jvm.clone()).unwrap_or_default(),
                overlay_game: Vec::new(),
                overlay_minecraft_arguments: None,
                extra_vars: Vec::new(),
            })
        }

        LoaderPrep::Overlay { json, ver_name } => {
            let kind = req.loader.short_label();
            let main_class = json["mainClass"].as_str()
                .ok_or_else(|| anyhow!("No mainClass in {kind} version JSON"))?
                .to_string();
            let classpath = download_overlay_libraries(ctx, json, libs_dir, kind, ver_name, req.concurrent_downloads).await?;
            Ok(LaunchPlan {
                version_name: ver_name.clone(),
                main_class: Some(main_class),
                classpath,
                overlay_jvm: json["arguments"]["jvm"].as_array().cloned().unwrap_or_default(),
                overlay_game: json["arguments"]["game"].as_array().cloned().unwrap_or_default(),
                overlay_minecraft_arguments: json["minecraftArguments"].as_str().map(|s| s.to_string()),
                extra_vars: vec![(
                    "${library_directory}".to_string(),
                    libs_dir.to_string_lossy().into_owned(),
                )],
            })
        }

        LoaderPrep::Lb { manifest } => {
            let loader_ver = &manifest.build.fabric_loader_version;
            let profile = fetch_meta_profile(ctx, "https://meta.fabricmc.net/v2",
                mc_ver, loader_ver, "Fabric").await?;
            let classpath = download_profile_libraries(ctx, &profile, None, "Fabric", req.concurrent_downloads).await?;
            download_lb_mods(ctx, manifest).await?;
            download_lb_extra_mods(ctx, mc_ver).await;

            Ok(LaunchPlan {
                version_name: format!("fabric-loader-{loader_ver}-{mc_ver}"),
                main_class: Some(profile.main_class.clone()),
                classpath,
                overlay_jvm: profile.arguments.as_ref().map(|a| a.jvm.clone()).unwrap_or_default(),
                overlay_game: Vec::new(),
                overlay_minecraft_arguments: None,
                extra_vars: Vec::new(),
            })
        }
    }
}
