//! The per-instance installed-mod index (`mods/.mlbv-mods.json`).
use super::ModMeta;
use crate::launcher::{instances_dir, valid_instance_name};
use std::collections::HashMap;
use std::path::PathBuf;

// ── installed-mod index ───────────────────────────────────────────────────────

pub(super) fn mods_dir(instance: &str) -> PathBuf {
    instances_dir().join(instance).join("mods")
}

pub(super) fn index_path(instance: &str) -> PathBuf {
    mods_dir(instance).join(".mlbv-mods.json")
}

pub(super) fn read_index(instance: &str) -> HashMap<String, ModMeta> {
    std::fs::read_to_string(index_path(instance))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub(super) fn write_index(instance: &str, index: &HashMap<String, ModMeta>) -> Result<(), String> {
    let dir = mods_dir(instance);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
    std::fs::write(index_path(instance), text).map_err(|e| e.to_string())
}

/// Drop index entries (used when files are deleted outside this module).
pub(crate) fn forget_mods(instance: &str, filenames: &[String]) {
    if valid_instance_name(instance).is_err() {
        return;
    }
    let mut index = read_index(instance);
    let before = index.len();
    for f in filenames {
        index.remove(f);
    }
    if index.len() != before {
        let _ = write_index(instance, &index);
    }
}
