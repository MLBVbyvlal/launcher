//! Per-process game state: running children, per-instance download controls,
//! JVM output buffers, and the RAII guards the pipeline uses.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64};

// ─── Game process state (shared across launch / stop commands) ───────────────

/// Game processes and their live output, keyed by instance name. Several
/// instances can run at the same time; each one is tracked on its own.
pub struct GameState {
    pub children:    Mutex<HashMap<String, std::process::Child>>,
    pub jvm_buffers: Mutex<HashMap<String, Arc<Mutex<Vec<String>>>>>,
    /// Download controls of every launch in flight, keyed by instance name.
    /// Pause/cancel address one launch; the entry is removed when it ends.
    pub downloads:   Mutex<HashMap<String, Arc<DlControl>>>,
    /// One lock per Minecraft version: the shared cache (client JAR,
    /// libraries, assets) is written by at most one launch at a time, so two
    /// instances of the same version never race on the same `.part` files.
    pub version_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}
impl GameState {
    pub fn new() -> Self {
        GameState {
            children:      Mutex::new(HashMap::new()),
            jvm_buffers:   Mutex::new(HashMap::new()),
            downloads:     Mutex::new(HashMap::new()),
            version_locks: Mutex::new(HashMap::new()),
        }
    }

    /// Register a fresh control block for `instance` (replacing a stale one).
    pub fn begin_download(&self, instance: &str) -> Arc<DlControl> {
        let ctl = Arc::new(DlControl::default());
        if let Ok(mut m) = self.downloads.lock() { m.insert(instance.to_string(), ctl.clone()); }
        ctl
    }
    pub fn end_download(&self, instance: &str) {
        if let Ok(mut m) = self.downloads.lock() { m.remove(instance); }
    }
    pub fn download_control(&self, instance: &str) -> Option<Arc<DlControl>> {
        self.downloads.lock().ok().and_then(|m| m.get(instance).cloned())
    }
    /// Names of instances that currently have a launch in flight.
    pub fn active_downloads(&self) -> Vec<String> {
        self.downloads.lock().map(|m| m.keys().cloned().collect()).unwrap_or_default()
    }
    pub(super) fn version_lock(&self, mc_version: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut m = self.version_locks.lock().unwrap_or_else(|e| e.into_inner());
        m.entry(mc_version.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

/// Pause / cancel flags and the byte counter of one launch.
#[derive(Default)]
pub struct DlControl {
    pub cancel: AtomicBool,
    pub pause:  AtomicBool,
    pub bytes:  AtomicU64,
}

/// Removes the control block when the launch ends, however it ends.
pub(super) struct DownloadGuard<'a> { pub(super) state: &'a GameState, pub(super) instance: String }
impl Drop for DownloadGuard<'_> {
    fn drop(&mut self) { self.state.end_download(&self.instance); }
}

// RAII guard that aborts a tokio task on drop
pub(super) struct AbortOnDrop(pub(super) tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) { self.0.abort(); }
}
