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
    let mc_version =
        std::env::var("MLBV_PROOF_MC_VERSION").unwrap_or_else(|_| MC_VERSION.to_string());

    // A real app handle: Builder::build creates the actual main window, so
    // this needs a display (xvfb in CI). The window itself is never touched —
    // only the handle, the managed state and the event emitter are used.
    let app = tauri::Builder::default()
        .manage(GameState::new())
        .build(tauri::generate_context!())
        .expect("build Tauri app (needs a display — use xvfb-run when headless)");
    let handle = app.handle().clone();

    launcher::launch(
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
    .expect("launch pipeline");

    // The pipeline spawned the game; wait until the client proves it booted.
    let log_path = launcher::instances_dir()
        .join(INSTANCE)
        .join("logs")
        .join("latest.log");
    let start = Instant::now();
    let booted = loop {
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        if log.contains(BOOT_MARKER) {
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
            println!("--- game exited early, full log ---\n{log}");
            break false;
        }
        if start.elapsed() > BOOT_TIMEOUT {
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
        "Minecraft did not print its LWJGL banner within {BOOT_TIMEOUT:?} — see the log above"
    );
}
