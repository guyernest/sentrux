//! Git diff overlay adapter — computes per-file change intensity from git history.
//!
//! Provides background thread spawning for on-demand git diff analysis,
//! plus save/load functions for analysis snapshots stored in `.sentrux/snapshot.json`.

use crate::core::pmat_types::{AnalysisSnapshot, GitDiffReport};
use crate::metrics::evo::git_walker::{walk_git_log_windowed, DiffWindow};

// ── Background thread spawning ────────────────────────────────────────────

/// Compute a `GitDiffReport` for the given root and window.
///
/// Calls `walk_git_log_windowed`, then aggregates results into a `GitDiffReport`.
/// Called from a background thread spawned by `draw_panels::maybe_spawn_git_diff_thread`.
pub fn compute_git_diff_report(root: &str, window: DiffWindow) -> Result<GitDiffReport, String> {
    let root_path = std::path::Path::new(root);
    let walk = walk_git_log_windowed(root_path, window)?;
    Ok(GitDiffReport::from_walk(walk.records, walk.new_file_paths, window))
}

// ── Analysis snapshot persistence ────────────────────────────────────────

/// Save an analysis snapshot to `.sentrux/snapshot.json` in the given root directory.
///
/// Creates the `.sentrux/` directory if it does not exist.
pub fn save_analysis_snapshot(root: &str, snapshot: &AnalysisSnapshot) -> Result<(), String> {
    let sentrux_dir = std::path::Path::new(root).join(".sentrux");
    std::fs::create_dir_all(&sentrux_dir)
        .map_err(|e| format!("Failed to create .sentrux directory: {e}"))?;
    let path = sentrux_dir.join("snapshot.json");
    let json = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("Failed to serialize snapshot: {e}"))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write snapshot to {}: {e}", path.display()))?;
    Ok(())
}

/// Load an analysis snapshot from git history at or before `boundary_epoch`.
///
/// Walks git history (up to 1000 commits) to find the most recent `.sentrux/snapshot.json`
/// blob whose commit timestamp is ≤ `boundary_epoch`. Returns `None` if no such snapshot
/// exists or if any git operation fails.
pub fn load_snapshot_at_boundary(root: &str, boundary_epoch: i64) -> Option<AnalysisSnapshot> {
    let repo = git2::Repository::discover(root).ok()?;
    let mut revwalk = repo.revwalk().ok()?;
    revwalk.set_sorting(git2::Sort::TIME).ok()?;
    revwalk.push_head().ok()?;

    let mut checked = 0usize;
    for oid_result in revwalk {
        if checked >= 1000 {
            break;
        }
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => { checked += 1; continue; }
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => { checked += 1; continue; }
        };
        checked += 1;

        if commit.time().seconds() > boundary_epoch {
            continue;
        }

        // Look for `.sentrux/snapshot.json` in this commit's tree
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let entry = match tree.get_path(std::path::Path::new(".sentrux/snapshot.json")) {
            Ok(e) => e,
            Err(_) => continue, // commit doesn't have snapshot file — expected
        };
        let blob = match repo.find_blob(entry.id()) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let content = match std::str::from_utf8(blob.content()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(snapshot) = serde_json::from_str::<AnalysisSnapshot>(content) {
            return Some(snapshot);
        }
    }

    None
}

// ── Unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::evo::git_walker::DiffWindow;
    use std::collections::HashMap;

    #[test]
    fn compute_git_diff_report_invalid_root_returns_err() {
        let result = compute_git_diff_report("/nonexistent/repo", DiffWindow::TimeSecs(3600));
        assert!(result.is_err(), "should fail for invalid repo path");
    }

    #[test]
    fn git_diff_report_is_valid_struct() {
        let report = GitDiffReport {
            by_file: HashMap::new(),
            max_intensity: 1.0,
            window: DiffWindow::default(),
            computed_at: 0,
        };
        assert_eq!(report.max_intensity, 1.0);
        assert!(report.by_file.is_empty());
        assert_eq!(report.window, DiffWindow::TimeSecs(86400));
    }
}
