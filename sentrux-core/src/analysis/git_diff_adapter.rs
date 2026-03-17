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
    let walk = walk_git_log_windowed(root_path, window.clone())?;
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
    use crate::core::pmat_types::{FileDiffData, FileAnalysisSnapshot};
    use crate::metrics::evo::git_walker::{CommitRecord, CommitFile, DiffWindow};
    use std::collections::{HashMap, HashSet};

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // ── compute_git_diff_report ──────────────────────────────────────────

    #[test]
    fn compute_git_diff_report_invalid_root_returns_err() {
        let result = compute_git_diff_report("/nonexistent/repo", DiffWindow::TimeSecs(3600));
        assert!(result.is_err(), "should fail for invalid repo path");
    }

    #[test]
    fn compute_git_diff_report_on_real_repo() {
        // Use the sentrux repo itself — should succeed
        let result = compute_git_diff_report(".", DiffWindow::CommitCount(1));
        assert!(result.is_ok(), "should succeed on a real git repo: {:?}", result.err());
        let report = result.unwrap();
        assert!(report.computed_at > 0, "should have a non-zero epoch");
    }

    // ── GitDiffReport::from_walk ─────────────────────────────────────────

    #[test]
    fn from_walk_empty_records() {
        let report = GitDiffReport::from_walk(vec![], HashSet::new(), DiffWindow::default());
        assert!(report.by_file.is_empty());
        assert_eq!(report.max_intensity, 1.0, "empty report defaults to 1.0 (no div by zero)");
    }

    #[test]
    fn from_walk_aggregates_multiple_commits() {
        let records = vec![
            CommitRecord {
                author: "alice".into(), epoch: 100,
                files: vec![
                    CommitFile { path: "src/main.rs".into(), added: 10, removed: 2 },
                    CommitFile { path: "src/lib.rs".into(), added: 5, removed: 0 },
                ],
            },
            CommitRecord {
                author: "bob".into(), epoch: 200,
                files: vec![
                    CommitFile { path: "src/main.rs".into(), added: 3, removed: 1 },
                ],
            },
        ];
        let report = GitDiffReport::from_walk(records, HashSet::new(), DiffWindow::TimeSecs(3600));

        let main = report.by_file.get("src/main.rs").expect("main.rs should exist");
        assert_eq!(main.commit_count, 2, "touched in 2 commits");
        assert_eq!(main.lines_added, 13, "10 + 3");
        assert_eq!(main.lines_removed, 3, "2 + 1");
        assert!(!main.is_new_file);

        let lib = report.by_file.get("src/lib.rs").expect("lib.rs should exist");
        assert_eq!(lib.commit_count, 1);
        assert_eq!(lib.lines_added, 5);
    }

    #[test]
    fn from_walk_marks_new_files() {
        let records = vec![
            CommitRecord {
                author: "alice".into(), epoch: 100,
                files: vec![
                    CommitFile { path: "src/new.rs".into(), added: 50, removed: 0 },
                    CommitFile { path: "src/old.rs".into(), added: 1, removed: 1 },
                ],
            },
        ];
        let new_files: HashSet<String> = ["src/new.rs".to_string()].into_iter().collect();
        let report = GitDiffReport::from_walk(records, new_files, DiffWindow::default());

        assert!(report.by_file["src/new.rs"].is_new_file, "new.rs should be marked as new");
        assert!(!report.by_file["src/old.rs"].is_new_file, "old.rs is not new");
    }

    #[test]
    fn from_walk_max_intensity_computed() {
        let records = vec![
            CommitRecord {
                author: "alice".into(), epoch: 100,
                files: vec![
                    CommitFile { path: "big.rs".into(), added: 100, removed: 50 },
                    CommitFile { path: "small.rs".into(), added: 1, removed: 0 },
                ],
            },
        ];
        let report = GitDiffReport::from_walk(records, HashSet::new(), DiffWindow::default());
        let big_intensity = report.by_file["big.rs"].raw_intensity();
        assert_eq!(report.max_intensity, big_intensity, "max_intensity should match the hottest file");
        assert!(report.max_intensity > 1.0, "should be > 1.0 for significant changes");
    }

    // ── FileDiffData::raw_intensity ──────────────────────────────────────

    #[test]
    fn raw_intensity_zero_for_no_changes() {
        let d = FileDiffData { commit_count: 0, lines_added: 0, lines_removed: 0, is_new_file: false };
        assert_eq!(d.raw_intensity(), 0.0);
    }

    #[test]
    fn raw_intensity_geometric_mean() {
        let d = FileDiffData { commit_count: 4, lines_added: 10, lines_removed: 6, is_new_file: false };
        // sqrt((10 + 6) * 4) = sqrt(64) = 8.0
        assert_eq!(d.raw_intensity(), 8.0);
    }

    #[test]
    fn raw_intensity_single_commit_single_line() {
        let d = FileDiffData { commit_count: 1, lines_added: 1, lines_removed: 0, is_new_file: false };
        assert_eq!(d.raw_intensity(), 1.0);
    }

    // ── save_analysis_snapshot ────────────────────────────────────────────

    #[test]
    fn save_snapshot_creates_file() {
        let tmp = temp_dir();
        let root = tmp.path().to_string_lossy().to_string();
        let snapshot = AnalysisSnapshot {
            computed_at: 12345,
            commit_sha: "abc".into(),
            files: vec![FileAnalysisSnapshot {
                path: "src/main.rs".into(),
                tdg_grade: Some("B".into()),
                coverage_pct: Some(80.0),
                clippy_count: Some(3),
            }],
        };
        let result = save_analysis_snapshot(&root, &snapshot);
        assert!(result.is_ok(), "save should succeed: {:?}", result.err());

        let path = tmp.path().join(".sentrux").join("snapshot.json");
        assert!(path.exists(), "snapshot.json should be created");

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: AnalysisSnapshot = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.computed_at, 12345);
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].path, "src/main.rs");
    }

    #[test]
    fn save_snapshot_creates_sentrux_dir() {
        let tmp = temp_dir();
        let root = tmp.path().to_string_lossy().to_string();
        let snapshot = AnalysisSnapshot {
            computed_at: 1, commit_sha: String::new(), files: vec![],
        };
        let _ = save_analysis_snapshot(&root, &snapshot);
        assert!(tmp.path().join(".sentrux").is_dir(), ".sentrux dir should be created");
    }

    #[test]
    fn save_snapshot_overwrites_existing() {
        let tmp = temp_dir();
        let root = tmp.path().to_string_lossy().to_string();
        let snap1 = AnalysisSnapshot { computed_at: 1, commit_sha: String::new(), files: vec![] };
        let snap2 = AnalysisSnapshot { computed_at: 2, commit_sha: String::new(), files: vec![] };
        save_analysis_snapshot(&root, &snap1).unwrap();
        save_analysis_snapshot(&root, &snap2).unwrap();

        let content = std::fs::read_to_string(tmp.path().join(".sentrux/snapshot.json")).unwrap();
        let loaded: AnalysisSnapshot = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.computed_at, 2, "second save should overwrite first");
    }

    // ── load_snapshot_at_boundary ─────────────────────────────────────────

    #[test]
    fn load_snapshot_at_boundary_no_repo() {
        let tmp = temp_dir();
        let result = load_snapshot_at_boundary(&tmp.path().to_string_lossy(), i64::MAX);
        assert!(result.is_none(), "should return None for non-git directory");
    }

    #[test]
    fn load_snapshot_at_boundary_repo_without_snapshot() {
        // Use the real repo but with epoch 0 — no commits predating epoch 0
        let result = load_snapshot_at_boundary(".", 0);
        assert!(result.is_none(), "should return None when no commit predates epoch 0");
    }

    // ── GitDiffReport struct ─────────────────────────────────────────────

    #[test]
    fn git_diff_report_default_window() {
        let report = GitDiffReport {
            by_file: HashMap::new(),
            max_intensity: 1.0,
            window: DiffWindow::default(),
            computed_at: 0,
        };
        assert_eq!(report.window, DiffWindow::TimeSecs(86400));
    }

    #[test]
    fn git_diff_report_commit_range_window() {
        let report = GitDiffReport {
            by_file: HashMap::new(),
            max_intensity: 1.0,
            window: DiffWindow::CommitRange { from: "abc".into(), to: "HEAD".into() },
            computed_at: 0,
        };
        assert_eq!(report.window, DiffWindow::CommitRange { from: "abc".into(), to: "HEAD".into() });
    }
}
