//! Manual launch proof: boots a real Minecraft client through the actual
//! `launcher::launch` pipeline (the same code the Play button runs) and
//! asserts the JVM gets far enough to print its LWJGL banner into
//! `latest.log`. Ignored by default — `cargo test` skips it; run it via the
//! `launch-proof` workflow, or by hand under xvfb with network access:
//! `cargo test --test headless_launch -- --ignored --nocapture`.
//!
//! Needs: a display (real or xvfb — the test builds a real Tauri app),
//! internet (Mojang + Adoptium), ~2 GB of disk for the shared cache and a few
//! minutes on first run. MC version via `MLBV_PROOF_MC_VERSION` (default below).
//!
//! Failure diagnostics are printed as GitHub workflow commands (`::notice`,
//! `::error`) because raw CI logs live on blob storage that restricted
//! sandboxes cannot reach, while annotations are served from the API —
//! every stage and every failure leaves one.
use std::time::{Duration, Instant};
use tauri::Manager;
use tauri_app_lib::launcher::{self, GameState, LaunchRequest, Loader};

const MC_VERSION: &str = "1.20.1";
const INSTANCE: &str = "ci-proof";
const BOOT_MARKER: &str = "LWJGL Version";
const BOOT_TIMEOUT: Duration = Duration::from_secs(600);

fn log_tail(log: &str, n: usize) -> String {
    let lines: Vec<&str> = log.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// Escape text for a GitHub workflow command payload.
fn esc(s: &str) -> String {
    s.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A")
}

fn note(title: &str, msg: &str) {
    let msg: String = msg.chars().take(600).collect();
    println!("::notice title={}::{}", esc(title), esc(&msg));
}

fn err_note(title: &str, msg: &str) {
    let msg: String = msg.chars().take(600).collect();
    println!("::error title={}::{}", esc(title), esc(&msg));
}

#[test]
#[ignore]
fn vanilla_client_boots_headless() {
    // No tokio macros in the dependency tree: the "macros" feature would pull
    // in tokio-macros and force a Cargo.lock update, so drive the async body
    // with an explicitly built current-thread runtime instead. Everything the
    // pipeline needs (spawn, sleep, blocking threads) works on it.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(proof_body());
}

async fn proof_body() {
    note(
        "proof-env",
        &format!(
            "DISPLAY={:?} HOME={:?} version={}",
            std::env::var("DISPLAY"),
            std::env::var("HOME"),
            std::env::var("MLBV_PROOF_MC_VERSION").unwrap_or_else(|_| MC_VERSION.to_string())
        ),
    );
    let mc_version =
        std::env::var("MLBV_PROOF_MC_VERSION").unwrap_or_else(|_| MC_VERSION.to_string());

    note("proof-stage", "building Tauri app");
    let app = match tauri::Builder::default()
        .manage(GameState::new())
        .build(tauri::generate_context!())
    {
        Ok(app) => app,
        Err(e) => {
            err_note("app-build-failed", &e.to_string());
            panic!("build Tauri app: {e}");
        }
    };
    let handle = app.handle().clone();

    note("proof-stage", "app built, starting launch pipeline");
    let t0 = Instant::now();
    if let Err(e) = launcher::launch(
        handle,
        LaunchRequest {
            loader: Loader::Vanilla,
            mc_version,
            loader_version: String::new(),
            lb_build_id: 0,
            instance_name: INSTANCE.to_string(),
            username: "CIPlayer".to_string(),
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
            offline: true,
            access_token: "0".to_string(),
            concurrent_downloads: 8,
            max_ram_mb: 2048,
        },
    )
    .await
    {
        err_note("pipeline-failed", &format!("{e:#}"));
        panic!("launch pipeline: {e:#}");
    }
    note(
        "proof-stage",
        &format!(
            "pipeline ok in {:.0}s, waiting for boot marker",
            t0.elapsed().as_secs()
        ),
    );

    // The pipeline spawned the game; wait until the client proves it booted.
    let log_path = launcher::instances_dir()
        .join(INSTANCE)
        .join("logs")
        .join("latest.log");
    let start = Instant::now();
    let booted = loop {
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        if let Some(marker) = log.lines().find(|l| l.contains(BOOT_MARKER)) {
            note("boot-marker", marker);
            println!("--- boot marker found, log tail ---\n{}", log_tail(&log, 15));
            break true;
        }
        // Fail fast if the game already died without booting.
        let running = app
            .state::<GameState>()
            .children
            .lock()
            .unwrap()
            .contains_key(INSTANCE);
        if !running {
            err_note("game-exited-early", &log_tail(&log, 25));
            println!("--- game exited early, full log ---\n{log}");
            break false;
        }
        if start.elapsed() > BOOT_TIMEOUT {
            err_note("boot-timeout", &log_tail(&log, 25));
            println!("--- boot timeout, log tail ---\n{}", log_tail(&log, 40));
            break false;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };

    // Stop the game; the proof is the marker, not a running process.
    if let Some(mut child) = app
        .state::<GameState>()
        .children
        .lock()
        .unwrap()
        .remove(INSTANCE)
    {
        let _ = child.kill();
    }

    assert!(
        booted,
        "Minecraft did not print its LWJGL banner within {BOOT_TIMEOUT:?} — see the annotations above"
    );
}
