//! 0.0.5 data migration: detect legacy layouts, report garbage, clean up.
use crate::launcher;

// ── 0.0.5 data migration ─────────────────────────────────────────────────────
// ≤0.0.4 extracted per-version natives into shared/versions/<mc>/natives — one
// folder shared (and never cleaned) by the vanilla, Fabric, Quilt, Forge and
// NeoForge pipelines — and duplicated jars/jsons per instance into
// instances/<name>/versions/. 0.0.5 extracts natives per instance (Prism-style)
// and keeps jars/jsons shared-only, so those two directory shapes are dead
// weight. Everything else (libraries, assets, installers, Java) is
// layout-identical and must be kept, not re-downloaded.
//
// The marker records which app version the on-disk data belongs to. Old
// releases never wrote it, so a missing marker + existing data = legacy.

#[tauri::command]
pub(crate) fn get_data_version() -> String {
    std::fs::read_to_string(launcher::mlbv_base().join("dataversion.txt"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[tauri::command]
pub(crate) fn set_data_version() -> Result<(), String> {
    let base = launcher::mlbv_base();
    // Fresh installs have no data dir yet — the marker must still stick.
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    std::fs::write(base.join("dataversion.txt"), env!("CARGO_PKG_VERSION"))
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub(crate) struct MigrationGarbage {
    /// Display path relative to the data dir (or temp dir), forward slashes.
    path: String,
    bytes: u64,
    /// One of: natives | inst_versions | part | temp.
    kind: &'static str,
}

#[derive(serde::Serialize)]
pub(crate) struct MigrationScan {
    garbage: Vec<MigrationGarbage>,
    total_bytes: u64,
    has_instances: bool,
    has_versions: bool,
}

fn dir_size(path: &std::path::Path, depth: u8) -> u64 {
    if depth == 0 { return 0; }
    let Ok(rd) = std::fs::read_dir(path) else { return 0; };
    rd.flatten().map(|e| {
        let p = e.path();
        if p.is_dir() { dir_size(&p, depth - 1) }
        else { e.metadata().map(|m| m.len()).unwrap_or(0) }
    }).sum()
}

fn display_rel(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

/// Collect *.part / .*.part files under `dir` (bounded depth).
fn collect_parts(dir: &std::path::Path, depth: u8, out: &mut Vec<std::path::PathBuf>) {
    if depth == 0 { return; }
    let Ok(rd) = std::fs::read_dir(dir) else { return; };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            // Instance game dirs hold user data (saves/mods); a *.part there is
            // still a leftover, but cap the walk so huge trees stay cheap.
            collect_parts(&p, depth - 1, out);
        } else if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
            if n.ends_with(".part") {
                out.push(p);
            }
        }
    }
}

fn dir_has_entries(path: &std::path::Path) -> bool {
    std::fs::read_dir(path).map(|mut rd| rd.next().is_some()).unwrap_or(false)
}

#[tauri::command]
pub(crate) fn migration_scan() -> MigrationScan {
    let base = launcher::mlbv_base();
    let mut garbage: Vec<MigrationGarbage> = Vec::new();

    // 1. Stale shared natives: shared/versions/*/natives (0.0.5 never creates
    //    a natives/ dir under versions/ — natives live per instance now).
    if let Ok(rd) = std::fs::read_dir(base.join("shared").join("versions")) {
        for e in rd.flatten() {
            let nat = e.path().join("natives");
            if nat.is_dir() {
                garbage.push(MigrationGarbage {
                    path: display_rel(&nat, &base),
                    bytes: dir_size(&nat, 6),
                    kind: "natives",
                });
            }
        }
    }

    // 2. Stale per-instance version dups: instances/*/versions (jars/jsons are
    //    shared-only since 0.0.5; instance game data is NOT touched).
    if let Ok(rd) = std::fs::read_dir(base.join("instances")) {
        for e in rd.flatten() {
            if !e.path().is_dir() { continue; }
            let vd = e.path().join("versions");
            if vd.is_dir() {
                garbage.push(MigrationGarbage {
                    path: display_rel(&vd, &base),
                    bytes: dir_size(&vd, 8),
                    kind: "inst_versions",
                });
            }
        }
    }

    // 3. Interrupted-download leftovers (*.part) anywhere under the data dir.
    let mut parts = Vec::new();
    collect_parts(&base, 10, &mut parts);
    for p in parts {
        let bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        garbage.push(MigrationGarbage { path: display_rel(&p, &base), bytes, kind: "part" });
    }

    // 4. Stale self-update packages in the temp dir (outside the data dir).
    let tmp = std::env::temp_dir();
    let name = "mlbv-just-updated.txt";
    let p = tmp.join(name);
    if p.is_file() {
        let bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        garbage.push(MigrationGarbage { path: format!("(temp)/{name}"), bytes, kind: "temp" });
    }
    if let Ok(rd) = std::fs::read_dir(&tmp) {
        for e in rd.flatten() {
            if let Some(n) = e.file_name().to_str() {
                if n.starts_with("mlbv-update.") && e.path().is_file() {
                    let bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
                    garbage.push(MigrationGarbage {
                        path: format!("(temp)/{n}"), bytes, kind: "temp",
                    });
                }
            }
        }
    }

    garbage.sort_by_key(|g| std::cmp::Reverse(g.bytes));
    let total_bytes = garbage.iter().map(|g| g.bytes).sum();
    MigrationScan {
        garbage,
        total_bytes,
        has_instances: dir_has_entries(&base.join("instances")),
        has_versions: dir_has_entries(&base.join("shared").join("versions")),
    }
}

#[derive(serde::Serialize)]
pub(crate) struct MigrationReport {
    deleted: Vec<String>,
    freed_bytes: u64,
    errors: Vec<String>,
}

/// Delete everything `migration_scan` reports. Paths are re-derived here from
/// the known roots — the frontend never supplies paths, so there is nothing
/// to escape or inject.
#[tauri::command]
pub(crate) fn migration_clean() -> MigrationReport {
    let base = launcher::mlbv_base();
    let mut deleted = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut freed_bytes: u64 = 0;

    let mut remove = |p: &std::path::Path, disp: String, is_dir: bool| {
        let bytes = if is_dir { dir_size(p, 12) } else { std::fs::metadata(p).map(|m| m.len()).unwrap_or(0) };
        let res = if is_dir { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
        match res {
            Ok(()) => { freed_bytes += bytes; deleted.push(disp); }
            Err(e) => errors.push(format!("{disp}: {e}")),
        }
    };

    if let Ok(rd) = std::fs::read_dir(base.join("shared").join("versions")) {
        for e in rd.flatten() {
            let nat = e.path().join("natives");
            if nat.is_dir() { remove(&nat, display_rel(&nat, &base), true); }
        }
    }
    if let Ok(rd) = std::fs::read_dir(base.join("instances")) {
        for e in rd.flatten() {
            if !e.path().is_dir() { continue; }
            let vd = e.path().join("versions");
            if vd.is_dir() { remove(&vd, display_rel(&vd, &base), true); }
        }
    }
    let mut parts = Vec::new();
    collect_parts(&base, 10, &mut parts);
    for p in parts {
        remove(&p, display_rel(&p, &base), false);
    }
    let tmp = std::env::temp_dir();
    let p = tmp.join("mlbv-just-updated.txt");
    if p.is_file() { remove(&p, "(temp)/mlbv-just-updated.txt".to_string(), false); }
    if let Ok(rd) = std::fs::read_dir(&tmp) {
        for e in rd.flatten() {
            if let Some(n) = e.file_name().to_str() {
                if n.starts_with("mlbv-update.") && e.path().is_file() {
                    remove(&e.path(), format!("(temp)/{n}"), false);
                }
            }
        }
    }

    // Sweep version dirs left empty by the cleanup (deepest first) — a dir that
    // held only stale natives is re-downloaded cleanly on the next launch.
    // Instance trees are deliberately never swept: an empty game subdir is
    // still the user's data placeholder, not ours to remove.
    let mut empty_dirs: Vec<std::path::PathBuf> = Vec::new();
    fn collect_empty(dir: &std::path::Path, depth: u8, out: &mut Vec<std::path::PathBuf>) {
        if depth == 0 { return; }
        let Ok(rd) = std::fs::read_dir(dir) else { return; };
        let mut any = false;
        for e in rd.flatten() {
            any = true;
            let p = e.path();
            if p.is_dir() { collect_empty(&p, depth - 1, out); }
        }
        if !any { out.push(dir.to_path_buf()); }
    }
    let versions_root = base.join("shared").join("versions");
    collect_empty(&versions_root, 4, &mut empty_dirs);
    empty_dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for d in empty_dirs {
        if d == versions_root { continue; }
        // Re-check: only delete when still empty (a parallel launch may write).
        if !dir_has_entries(&d) { let _ = std::fs::remove_dir(&d); }
    }

    MigrationReport { deleted, freed_bytes, errors }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_rel_uses_forward_slashes() {
        let root = std::path::Path::new("/data/mlbv");
        assert_eq!(
            display_rel(&root.join("shared/versions/1.21/natives"), root),
            "shared/versions/1.21/natives"
        );
        // Outside the root: full path, still slash-normalized.
        assert_eq!(display_rel(std::path::Path::new("/tmp/x"), root), "/tmp/x");
        assert!(!dir_has_entries(&root.join("does-not-exist")));
    }
}
