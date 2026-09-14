//! Spawning the game, tracking its exit, buffering JVM output.
use super::*;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

// ─── JVM output buffer ────────────────────────────────────────────────────────

/// Per-instance bound for live JVM output. Long sessions would otherwise grow
/// the buffer without limit; the full history always stays in latest.log.
pub(super) const JVM_LINES_CAP: usize = 20_000;

pub(super) fn jvm_push(buf: &Mutex<Vec<String>>, line: String) {
    let mut lines = buf.lock().unwrap();
    lines.push(line);
    if lines.len() > JVM_LINES_CAP {
        let excess = lines.len() - JVM_LINES_CAP;
        lines.drain(..excess);
    }
}

// ─── Process spawn and tracking ──────────────────────────────────────────────

/// Lines kept in memory per instance after the game exits, so the console
/// window and the crash dialog still have something to show.
pub(super) const JVM_TAIL_AFTER_EXIT: usize = 2_000;

pub(super) fn pipe_lines<R: std::io::Read + Send + 'static>(stream: Option<R>, buf: Arc<Mutex<Vec<String>>>) {
    let Some(stream) = stream else { return };
    std::thread::spawn(move || {
        use std::io::BufRead;
        std::io::BufReader::new(stream).lines().flatten().for_each(|l| jvm_push(&buf, l));
    });
}

/// Start the JVM, register it under the instance name and watch for exit.
/// Instances are tracked independently, so several can run at the same time.
pub(super) fn spawn_game(
    app: &tauri::AppHandle,
    java: &Path,
    args: &[String],
    game_dir: &Path,
    instance: &str,
) -> Result<()> {
    let state = app.state::<GameState>();

    let buffer = Arc::new(Mutex::new(Vec::new()));
    state.jvm_buffers.lock().unwrap()
        .insert(instance.to_string(), buffer.clone());

    let mut cmd = std::process::Command::new(java);
    cmd.args(args)
        .current_dir(game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn()
        .with_context(|| format!("Failed to start Java from {:?}", java))?;

    pipe_lines(child.stdout.take(), buffer.clone());
    pipe_lines(child.stderr.take(), buffer.clone());

    state.children.lock().unwrap().insert(instance.to_string(), child);
    let _ = app.emit("game-running", serde_json::json!({
        "instance": instance,
        "running": true,
    }));

    watch_exit(app.clone(), instance.to_string(), game_dir.to_path_buf());
    Ok(())
}

/// Poll one child until it exits, then emit `game-crashed` (non-zero exit) and
/// `game-running: false` for that instance.
pub(super) fn watch_exit(app: tauri::AppHandle, instance: String, game_dir: PathBuf) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));

            let status = {
                let state = app.state::<GameState>();
                let mut children = state.children.lock().unwrap();
                match children.get_mut(&instance) {
                    Some(child) => match child.try_wait() {
                        Ok(status) => status,
                        Err(_) => None,
                    },
                    // Removed by stop_game — it emits its own event.
                    None => return,
                }
            };
            let Some(status) = status else { continue };

            {
                let state = app.state::<GameState>();
                state.children.lock().unwrap().remove(&instance);
            }

            if !status.success() {
                let log_path = game_dir.join("logs").join("latest.log");
                let log_tail = {
                    let state = app.state::<GameState>();
                    let buffers = state.jvm_buffers.lock().unwrap();
                    let captured = buffers.get(&instance)
                        .map(|b| b.lock().unwrap().clone())
                        .unwrap_or_default();
                    if !captured.is_empty() {
                        captured[captured.len().saturating_sub(80)..].join("\n")
                    } else {
                        fs::read_to_string(&log_path)
                            .map(|s| {
                                let v: Vec<&str> = s.lines().collect();
                                v[v.len().saturating_sub(80)..].join("\n")
                            })
                            .unwrap_or_else(|_| "No output captured.".into())
                    }
                };
                let _ = app.emit("game-crashed", serde_json::json!({
                    "instance": instance,
                    "exitCode": status.code().unwrap_or(-1),
                    "log": log_tail,
                    "logPath": log_path.to_string_lossy().into_owned(),
                }));
            }

            // Keep a tail for the console window, drop the rest of the buffer.
            {
                let state = app.state::<GameState>();
                let buffers = state.jvm_buffers.lock().unwrap();
                if let Some(buf) = buffers.get(&instance) {
                    let mut lines = buf.lock().unwrap();
                    let excess = lines.len().saturating_sub(JVM_TAIL_AFTER_EXIT);
                    if excess > 0 { lines.drain(..excess); }
                }
            }

            let _ = app.emit("game-running", serde_json::json!({
                "instance": instance,
                "running": false,
            }));
            break;
        }
    });
}
