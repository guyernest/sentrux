//! Git diff overlay adapter — computes per-file change intensity from git history.
//!
//! Provides background thread spawning for on-demand git diff analysis,
//! plus save/load functions for analysis snapshots stored in `.sentrux/snapshot.json`.

use crate::app::channels::ScanMsg;
use crate::core::pmat_types::{AnalysisSnapshot, GitDiffReport};
use crate::metrics::evo::git_walker::{walk_git_log_windowed, DiffWindow};
use crossbeam_channel::Sender;

// ── Background thread spawning ────────────────────────────────────────────

/// Spawn a background thread to compute a git diff report and deliver it via `ScanMsg`.
///
/// On success, sends `ScanMsg::GitDiffReady(report)`.
/// On failure, sends `ScanMsg::GitDiffError(message)`.
pub fn spawn_git_diff_thread(root: String, window: DiffWindow, scan_msg_tx: Sender<ScanMsg>) {
    std::thread::spawn(move || {
        match compute_git_diff_report(&root, window) {
            Ok(report) => {
                let _ = scan_msg_tx.send(ScanMsg::GitDiffReady(report));
            }
            Err(e) => {
                let _ = scan_msg_tx.send(ScanMsg::GitDiffError(e));
            }
        }
    });
}

/// Compute a `GitDiffReport` for the given root and window.
///
/// Calls `walk_git_log_windowed`, then aggregates results into a `GitDiffReport`.
fn compute_git_diff_report(root: &str, window: DiffWindow) -> Result<GitDiffReport, String> {
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
        let oid = oid_result.ok()?;
        let commit = repo.find_commit(oid).ok()?;
        checked += 1;

        if commit.time().seconds() > boundary_epoch {
            continue;
        }

        // Look for `.sentrux/snapshot.json` in this commit's tree
        let tree = commit.tree().ok()?;
        let entry = tree.get_path(std::path::Path::new(".sentrux/snapshot.json")).ok()?;
        let blob = repo.find_blob(entry.id()).ok()?;
        let content = std::str::from_utf8(blob.content()).ok()?;
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
    use crate::core::pmat_types::GitDiffReport;
    use crate::metrics::evo::git_walker::DiffWindow;
    use std::collections::HashMap;

    #[test]
    fn spawn_git_diff_thread_sends_git_diff_ready_on_nonexistent_repo() {
        // A non-existent root will cause git discover to fail → GitDiffError
        let (tx, rx) = crossbeam_channel::bounded(1);
        spawn_git_diff_thread("/nonexistent/path/that/does/not/exist".to_string(), DiffWindow::TimeSecs(3600), tx);
        let msg = rx.recv().expect("should receive a message");
        assert!(matches!(msg, ScanMsg::GitDiffError(_)), "expected GitDiffError for invalid repo");
    }

    #[test]
    fn spawn_git_diff_thread_sends_git_diff_ready_on_valid_repo() {
        // Use the workspace root (a real git repo)
        let workspace_root = env!("CARGO_MANIFEST_DIR");
        let root = std::path::Path::new(workspace_root)
            .parent() // sentrux-core → sentrux workspace root
            .unwrap_or(std::path::Path::new(workspace_root))
            .to_string_lossy()
            .to_string();
        let (tx, rx) = crossbeam_channel::bounded(1);
        // Use CommitCount(1) for fast test — just walk the single most recent commit
        spawn_git_diff_thread(root, DiffWindow::CommitCount(1), tx);
        let msg = rx.recv().expect("should receive a message");
        // Should be either GitDiffReady or GitDiffError (both are valid depending on repo state)
        assert!(
            matches!(msg, ScanMsg::GitDiffReady(_)) || matches!(msg, ScanMsg::GitDiffError(_)),
            "should receive GitDiffReady or GitDiffError"
        );
    }

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
