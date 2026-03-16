//! Snapshot persistence for the timeline system.
//!
//! Provides functions to write, load, prune, and compute deltas between
//! `AnalysisSnapshot` files stored in `.sentrux/snapshots/`.
//!
//! Snapshots are named `{epoch}.json` and are written at scan completion.
//! The nearest snapshot predating a target epoch can be loaded for delta
//! computation against the current analysis state.

use crate::core::pmat_types::{
    AnalysisSnapshot, FileAnalysisSnapshot,
    FileDeltaEntry, TimelineDeltaReport,
    PmatReport, CoverageReport, ClippyReport,
    grade_delta,
};
use crate::metrics::evo::git_walker::epoch_now;
use std::path::PathBuf;

// ── Write ─────────────────────────────────────────────────────────────────

/// Write an analysis snapshot to `.sentrux/snapshots/{epoch}.json`.
///
/// Builds `AnalysisSnapshot` from the current analysis reports, creates the
/// snapshots directory if needed, serialises to JSON, and prunes to `max_count`.
///
/// Returns the path of the written file on success, or an error message.
pub fn write_analysis_snapshot(
    root: &str,
    pmat: &Option<PmatReport>,
    coverage: &Option<CoverageReport>,
    clippy: &Option<ClippyReport>,
) -> Result<String, String> {
    let epoch = epoch_now();

    // Build per-file entries from PMAT (primary source of file list)
    let mut files: Vec<FileAnalysisSnapshot> = Vec::new();
    if let Some(pmat) = pmat {
        for file_score in &pmat.tdg.files {
            let path = file_score.file_path.trim_start_matches("./").to_string();
            let coverage_pct = coverage
                .as_ref()
                .and_then(|c| c.by_path.get(&path))
                .map(|&idx| coverage.as_ref().unwrap().files[idx].summary.lines.percent);
            let clippy_count = clippy
                .as_ref()
                .and_then(|c| c.by_file.get(&path))
                .map(|d| d.total);
            files.push(FileAnalysisSnapshot {
                path,
                tdg_grade: Some(file_score.grade.clone()),
                coverage_pct,
                clippy_count,
            });
        }
    }

    let snapshot = AnalysisSnapshot {
        computed_at: epoch,
        commit_sha: String::new(),
        files,
    };

    // Ensure snapshots directory exists
    let snapshots_dir = PathBuf::from(root).join(".sentrux").join("snapshots");
    std::fs::create_dir_all(&snapshots_dir)
        .map_err(|e| format!("Failed to create snapshots dir: {e}"))?;

    // Write snapshot file
    let filename = snapshots_dir.join(format!("{epoch}.json"));
    let json = serde_json::to_string(&snapshot)
        .map_err(|e| format!("Failed to serialize snapshot: {e}"))?;
    std::fs::write(&filename, json)
        .map_err(|e| format!("Failed to write snapshot {filename:?}: {e}"))?;

    let path_str = filename.to_string_lossy().to_string();

    // Prune old snapshots
    prune_snapshots(root, 50);

    Ok(path_str)
}

// ── Load ──────────────────────────────────────────────────────────────────

/// Load the snapshot whose epoch is the largest value <= `target_epoch`.
///
/// Returns `None` if the snapshots directory is empty, missing, or no snapshot
/// predates `target_epoch`.
pub fn load_nearest_snapshot(root: &str, target_epoch: i64) -> Option<AnalysisSnapshot> {
    let snapshots_dir = PathBuf::from(root).join(".sentrux").join("snapshots");

    let entries = std::fs::read_dir(&snapshots_dir).ok()?;

    let mut best_epoch: Option<i64> = None;
    let mut best_path: Option<PathBuf> = None;
    // Track oldest snapshot as fallback when target_epoch predates all snapshots
    let mut oldest_epoch: Option<i64> = None;
    let mut oldest_path: Option<PathBuf> = None;

    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        // Parse "{epoch}.json"
        if let Some(stem) = name.strip_suffix(".json") {
            if let Ok(ep) = stem.parse::<i64>() {
                // Track oldest snapshot
                if oldest_epoch.is_none() || ep < oldest_epoch.unwrap() {
                    oldest_epoch = Some(ep);
                    oldest_path = Some(entry.path());
                }
                if ep <= target_epoch {
                    if best_epoch.is_none() || ep > best_epoch.unwrap() {
                        best_epoch = Some(ep);
                        best_path = Some(entry.path());
                    }
                }
            }
        }
    }

    // Use the closest snapshot predating target_epoch; if none exists, fall back
    // to the oldest snapshot available (the earliest known state)
    let path = best_path.or(oldest_path)?;
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

// ── Delta computation ─────────────────────────────────────────────────────

/// Compute a delta report comparing the baseline snapshot to the current analysis.
///
/// For each file present in both the baseline and current PMAT report:
/// - `tdg_grade_delta`: new rank − old rank (positive = improved)
/// - `coverage_pct_delta`: new% − old% (positive = improved)
/// - `clippy_count_delta`: new count − old count (negative = improved)
///
/// Files only in baseline or only in current produce no entry (no comparison basis).
pub fn compute_delta_report(
    baseline: &AnalysisSnapshot,
    pmat: &Option<PmatReport>,
    coverage: &Option<CoverageReport>,
    clippy: &Option<ClippyReport>,
) -> TimelineDeltaReport {
    // Index baseline by path
    let baseline_map: std::collections::HashMap<&str, &FileAnalysisSnapshot> = baseline
        .files
        .iter()
        .map(|f| (f.path.as_str(), f))
        .collect();

    let mut by_file: std::collections::HashMap<String, FileDeltaEntry> =
        std::collections::HashMap::new();

    // Only compute deltas for files present in the current PMAT report
    if let Some(pmat) = pmat {
        for file_score in &pmat.tdg.files {
            let path = file_score.file_path.trim_start_matches("./");

            // Only produce an entry if the file was also in the baseline
            let baseline_entry = match baseline_map.get(path) {
                Some(e) => e,
                None => continue,
            };

            // TDG grade delta
            let old_grade = baseline_entry.tdg_grade.as_deref().unwrap_or("");
            let new_grade = file_score.grade.as_str();
            let tdg_grade_delta = grade_delta(old_grade, new_grade);

            // Coverage delta
            let new_coverage = coverage
                .as_ref()
                .and_then(|c| c.by_path.get(path))
                .map(|&idx| coverage.as_ref().unwrap().files[idx].summary.lines.percent);
            let coverage_pct_delta = match (baseline_entry.coverage_pct, new_coverage) {
                (Some(old), Some(new)) => Some(new - old),
                _ => None,
            };

            // Clippy count delta
            let new_clippy = clippy
                .as_ref()
                .and_then(|c| c.by_file.get(path))
                .map(|d| d.total);
            let clippy_count_delta = match (baseline_entry.clippy_count, new_clippy) {
                (Some(old), Some(new)) => Some(new as i32 - old as i32),
                _ => None,
            };

            by_file.insert(
                path.to_string(),
                FileDeltaEntry {
                    tdg_grade_delta,
                    coverage_pct_delta,
                    clippy_count_delta,
                },
            );
        }
    }

    TimelineDeltaReport {
        by_file,
        baseline_epoch: baseline.computed_at,
    }
}

// ── Prune ─────────────────────────────────────────────────────────────────

/// Delete the oldest snapshots until at most `max_count` remain.
///
/// Snapshots are sorted by their epoch filename and the oldest are removed first.
/// Silently ignores I/O errors (best-effort housekeeping).
pub fn prune_snapshots(root: &str, max_count: usize) {
    let snapshots_dir = PathBuf::from(root).join(".sentrux").join("snapshots");

    let entries = match std::fs::read_dir(&snapshots_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut snapshots: Vec<(i64, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let fname = entry.file_name();
            let name = fname.to_string_lossy().into_owned();
            let stem = name.strip_suffix(".json")?;
            let ep = stem.parse::<i64>().ok()?;
            Some((ep, entry.path()))
        })
        .collect();

    if snapshots.len() <= max_count {
        return;
    }

    // Sort ascending by epoch so we delete oldest first
    snapshots.sort_unstable_by_key(|(ep, _)| *ep);

    let to_delete = snapshots.len() - max_count;
    for (_, path) in snapshots.iter().take(to_delete) {
        let _ = std::fs::remove_file(path);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pmat_types::{
        AnalysisSnapshot, FileAnalysisSnapshot,
        PmatTdgOutput, PmatFileScore, PmatReport,
        CoverageOutput, CoverageDataSection, CoverageFileEntry,
        FileCoverageSummary, CoverageSummaryMetric,
        FileClippyData, ClippyReport,
    };
    use std::collections::HashMap;

    // ── Helpers ──────────────────────────────────────────────────────────

    fn temp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir should succeed")
    }

    fn make_pmat_with_file(path: &str, grade: &str) -> PmatReport {
        let file = PmatFileScore {
            file_path: format!("./{path}"),
            grade: grade.to_string(),
            total: 70.0,
            structural_complexity: 70.0,
            semantic_complexity: 70.0,
            duplication_ratio: 70.0,
            coupling_score: 70.0,
            doc_coverage: 70.0,
            consistency_score: 70.0,
            entropy_score: 70.0,
            confidence: 0.9,
            language: "rust".to_string(),
            critical_defects_count: 0,
            has_critical_defects: false,
            penalties_applied: vec![],
        };
        let tdg = PmatTdgOutput {
            files: vec![file],
            average_score: 70.0,
            average_grade: grade.to_string(),
            total_files: 1,
            language_distribution: HashMap::new(),
        };
        PmatReport::from_tdg(tdg, None)
    }

    fn make_snapshot_at(epoch: i64, file_path: &str, grade: &str) -> AnalysisSnapshot {
        AnalysisSnapshot {
            computed_at: epoch,
            commit_sha: String::new(),
            files: vec![FileAnalysisSnapshot {
                path: file_path.to_string(),
                tdg_grade: Some(grade.to_string()),
                coverage_pct: None,
                clippy_count: None,
            }],
        }
    }

    fn write_snapshot_at_epoch(dir: &std::path::Path, epoch: i64, content: &AnalysisSnapshot) {
        let snapshots_dir = dir.join(".sentrux").join("snapshots");
        std::fs::create_dir_all(&snapshots_dir).unwrap();
        let path = snapshots_dir.join(format!("{epoch}.json"));
        let json = serde_json::to_string(content).unwrap();
        std::fs::write(path, json).unwrap();
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_write_and_load_snapshot() {
        let tmp = temp_root();
        let root = tmp.path().to_string_lossy().to_string();
        let pmat = make_pmat_with_file("src/main.rs", "B");

        let written_path = write_analysis_snapshot(&root, &Some(pmat), &None, &None)
            .expect("write_analysis_snapshot should succeed");

        assert!(std::path::Path::new(&written_path).exists(), "Written snapshot file must exist");

        // Extract epoch from written path filename
        let fname = std::path::Path::new(&written_path)
            .file_name().unwrap()
            .to_string_lossy();
        let epoch: i64 = fname.strip_suffix(".json").unwrap().parse().unwrap();

        let loaded = load_nearest_snapshot(&root, epoch)
            .expect("load_nearest_snapshot should find the written snapshot");
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].path, "src/main.rs");
        assert_eq!(loaded.files[0].tdg_grade.as_deref(), Some("B"));
    }

    #[test]
    fn test_load_nearest_picks_closest() {
        let tmp = temp_root();
        let root = tmp.path().to_string_lossy().to_string();

        // Write 3 snapshots at epochs 100, 200, 300
        for &ep in &[100i64, 200, 300] {
            let snap = make_snapshot_at(ep, "src/foo.rs", "C");
            write_snapshot_at_epoch(tmp.path(), ep, &snap);
        }

        // load_nearest(250) should return epoch 200
        let loaded = load_nearest_snapshot(&root, 250)
            .expect("should find a snapshot at or before epoch 250");
        assert_eq!(loaded.computed_at, 200, "nearest snapshot to 250 should be epoch 200");
    }

    #[test]
    fn test_load_nearest_empty_dir() {
        let tmp = temp_root();
        let root = tmp.path().to_string_lossy().to_string();

        // Create the directory but put nothing in it
        std::fs::create_dir_all(tmp.path().join(".sentrux").join("snapshots")).unwrap();

        let result = load_nearest_snapshot(&root, 1_000_000);
        assert!(result.is_none(), "empty snapshots dir should return None");
    }

    #[test]
    fn test_load_nearest_all_newer_falls_back_to_oldest() {
        let tmp = temp_root();
        let root = tmp.path().to_string_lossy().to_string();

        // Write snapshots at epochs 500 and 1000 — both newer than our target of 100
        for &ep in &[500i64, 1000] {
            let snap = make_snapshot_at(ep, "src/lib.rs", "A");
            write_snapshot_at_epoch(tmp.path(), ep, &snap);
        }

        // Falls back to the oldest available snapshot (epoch 500)
        let result = load_nearest_snapshot(&root, 100);
        assert!(result.is_some(), "should fall back to oldest snapshot when all are newer");
        assert_eq!(result.unwrap().computed_at, 500);
    }

    #[test]
    fn test_prune_snapshots() {
        let tmp = temp_root();
        let root = tmp.path().to_string_lossy().to_string();

        // Write 5 snapshots at epochs 1..=5
        for ep in 1i64..=5 {
            let snap = make_snapshot_at(ep, "src/foo.rs", "C");
            write_snapshot_at_epoch(tmp.path(), ep, &snap);
        }

        // Prune to max 3 — epochs 1 and 2 (oldest) should be deleted
        prune_snapshots(&root, 3);

        let snapshots_dir = tmp.path().join(".sentrux").join("snapshots");
        let remaining: Vec<i64> = std::fs::read_dir(&snapshots_dir)
            .unwrap()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.strip_suffix(".json")?.parse().ok()
            })
            .collect();

        assert_eq!(remaining.len(), 3, "should have exactly 3 snapshots remaining");
        for ep in [1i64, 2] {
            assert!(
                !remaining.contains(&ep),
                "epoch {ep} should have been pruned (oldest)"
            );
        }
        for ep in [3i64, 4, 5] {
            assert!(
                remaining.contains(&ep),
                "epoch {ep} should still exist"
            );
        }
    }

    #[test]
    fn test_compute_delta_improved() {
        // Baseline: file has grade C; current: grade A → delta = +6
        let baseline = make_snapshot_at(100, "src/main.rs", "C");
        let pmat = make_pmat_with_file("src/main.rs", "A");

        let report = compute_delta_report(&baseline, &Some(pmat), &None, &None);
        let entry = report.by_file.get("src/main.rs")
            .expect("delta entry should exist for file in both baseline and current");
        assert_eq!(entry.tdg_grade_delta, 6, "C→A should be rank delta +6");
        assert!(entry.tdg_grade_delta > 0, "improvement should be positive");
    }

    #[test]
    fn test_compute_delta_regressed() {
        // Baseline: file has grade A; current: grade C → delta = -6
        let baseline = make_snapshot_at(100, "src/main.rs", "A");
        let pmat = make_pmat_with_file("src/main.rs", "C");

        let report = compute_delta_report(&baseline, &Some(pmat), &None, &None);
        let entry = report.by_file.get("src/main.rs")
            .expect("delta entry should exist");
        assert_eq!(entry.tdg_grade_delta, -6, "A→C should be rank delta -6");
        assert!(entry.tdg_grade_delta < 0, "regression should be negative");
    }

    #[test]
    fn test_compute_delta_new_file_not_in_baseline() {
        // File is in current pmat but not in baseline — no entry should be produced
        let baseline = AnalysisSnapshot {
            computed_at: 100,
            commit_sha: String::new(),
            files: vec![],
        };
        let pmat = make_pmat_with_file("src/new.rs", "B");

        let report = compute_delta_report(&baseline, &Some(pmat), &None, &None);
        assert!(
            !report.by_file.contains_key("src/new.rs"),
            "new file not in baseline should not produce a delta entry"
        );
    }

    #[test]
    fn test_compute_delta_baseline_epoch_set() {
        let baseline = make_snapshot_at(12345, "src/lib.rs", "B");
        let report = compute_delta_report(&baseline, &None, &None, &None);
        assert_eq!(report.baseline_epoch, 12345);
    }
}
