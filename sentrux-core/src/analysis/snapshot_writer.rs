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
use std::path::{Path, PathBuf};

// ── Write ─────────────────────────────────────────────────────────────────

/// Write an analysis snapshot to `.sentrux/snapshots/{epoch}.json`.
///
/// Skips the write if git HEAD hasn't changed since the most recent snapshot
/// (same code = same scores). This prevents watcher-triggered rescans from
/// flooding `.sentrux/snapshots/` with identical data.
///
/// Returns the path of the written file on success, `Ok("")` if skipped
/// (HEAD unchanged), or an error message.
pub fn write_analysis_snapshot(
    root: &str,
    pmat: &Option<PmatReport>,
    coverage: &Option<CoverageReport>,
    clippy: &Option<ClippyReport>,
) -> Result<String, String> {
    let head_sha = git_head_sha(root).unwrap_or_default();

    // Skip if HEAD hasn't changed since the most recent snapshot
    let mut snapshots = list_snapshot_epochs(root);
    let mut old_snapshot_to_replace: Option<PathBuf> = None;
    if !head_sha.is_empty() {
        if let Some((_, latest_path)) = snapshots.iter().max_by_key(|(ep, _)| *ep) {
            if let Some(latest) = load_snapshot_file(latest_path) {
                if latest.commit_sha == head_sha {
                    let had_coverage = latest.files.iter().any(|f| f.coverage_pct.is_some());
                    let has_coverage = coverage.is_some();
                    let had_clippy = latest.files.iter().any(|f| f.clippy_count.is_some());
                    let has_clippy = clippy.is_some();

                    if (!has_coverage || had_coverage) && (!has_clippy || had_clippy) {
                        return Ok(String::new()); // no new data — skip
                    }
                    old_snapshot_to_replace = Some(latest_path.clone());
                }
            }
        }
    }

    let epoch = epoch_now();

    // Build per-file entries from PMAT. PMAT may return absolute paths in by_path
    // keys (e.g. "/Users/.../sentrux-core/src/main.rs"), while coverage and clippy
    // use scan-root-relative paths (e.g. "sentrux-core/src/main.rs"). Normalize
    // by stripping the root prefix.
    let root_prefix = format!("{}/", root.trim_end_matches('/'));
    let mut files: Vec<FileAnalysisSnapshot> = Vec::new();
    if let Some(pmat) = pmat {
        for (raw_path, &idx) in &pmat.by_path {
            let rel_path = normalize_pmat_path(raw_path, &root_prefix);
            let grade = pmat.tdg.files[idx].grade.clone();
            let coverage_pct = coverage
                .as_ref()
                .and_then(|c| c.by_path.get(rel_path))
                .map(|&ci| coverage.as_ref().unwrap().files[ci].summary.lines.percent);
            let clippy_count = clippy
                .as_ref()
                .and_then(|c| c.by_file.get(rel_path))
                .map(|d| d.total);
            files.push(FileAnalysisSnapshot {
                path: rel_path.to_string(),
                tdg_grade: Some(grade),
                coverage_pct,
                clippy_count,
            });
        }
    }

    let snapshot = AnalysisSnapshot {
        computed_at: epoch,
        commit_sha: head_sha,
        files,
    };

    // Ensure snapshots directory exists
    let dir = snapshots_dir(root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create snapshots dir: {e}"))?;

    // Write new snapshot first, then delete old (safe order — never leaves zero files)
    let filename = dir.join(format!("{epoch}.json"));
    let json = serde_json::to_string(&snapshot)
        .map_err(|e| format!("Failed to serialize snapshot: {e}"))?;
    std::fs::write(&filename, json)
        .map_err(|e| format!("Failed to write snapshot {filename:?}: {e}"))?;

    // Delete old snapshot for same commit after new one is safely written
    if let Some(old_path) = old_snapshot_to_replace {
        let _ = std::fs::remove_file(&old_path);
    }

    let path_str = filename.to_string_lossy().to_string();

    // Prune using already-collected list (add the new file, avoids re-scanning directory)
    snapshots.push((epoch, filename));
    prune_snapshots_from_list(&mut snapshots, 50);

    Ok(path_str)
}

/// Normalize a PMAT path to scan-root-relative form.
/// PMAT may return absolute paths or `./`-prefixed paths; strip the root prefix
/// to match the relative keys used by coverage and clippy reports.
fn normalize_pmat_path<'a>(raw: &'a str, root_prefix: &str) -> &'a str {
    raw.strip_prefix(root_prefix)
        .or_else(|| raw.strip_prefix("./"))
        .unwrap_or(raw)
}

/// Get the SHA of the current git HEAD, or None if not in a git repo.
fn git_head_sha(root: &str) -> Option<String> {
    let repo = git2::Repository::discover(root).ok()?;
    let head = repo.head().ok()?;
    head.target().map(|oid| oid.to_string())
}

// ── Shared helpers ───────────────────────────────────────────────────────

/// Canonical snapshots directory path.
fn snapshots_dir(root: &str) -> PathBuf {
    PathBuf::from(root).join(".sentrux").join("snapshots")
}

/// List all snapshot files as (epoch, path) pairs, sorted ascending by epoch.
/// Single directory scan shared by load, prune, and write-dedup logic.
fn list_snapshot_epochs(root: &str) -> Vec<(i64, PathBuf)> {
    let entries = match std::fs::read_dir(snapshots_dir(root)) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut result: Vec<(i64, PathBuf)> = entries.flatten().filter_map(|entry| {
        let name = entry.file_name();
        let stem = name.to_string_lossy();
        let ep = stem.strip_suffix(".json")?.parse::<i64>().ok()?;
        Some((ep, entry.path()))
    }).collect();
    result.sort_unstable_by_key(|(ep, _)| *ep);
    result
}

/// Load a snapshot file from disk.
fn load_snapshot_file(path: &Path) -> Option<AnalysisSnapshot> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

// ── Load ──────────────────────────────────────────────────────────────────

/// Load the snapshot whose epoch is the largest value <= `target_epoch`.
///
/// Falls back to the oldest snapshot when `target_epoch` predates all snapshots
/// (returns the earliest known state rather than None).
pub fn load_nearest_snapshot(root: &str, target_epoch: i64) -> Option<AnalysisSnapshot> {
    let snapshots = list_snapshot_epochs(root); // sorted ascending
    if snapshots.is_empty() {
        return None;
    }

    // Find the largest epoch <= target_epoch via reverse search on sorted list
    let best = snapshots.iter().rev().find(|(ep, _)| *ep <= target_epoch);
    // Fall back to oldest (first element) when target predates all snapshots
    let (_, path) = best.unwrap_or(&snapshots[0]);
    load_snapshot_file(path)
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
    root: &str,
    baseline: &AnalysisSnapshot,
    pmat: &Option<PmatReport>,
    coverage: &Option<CoverageReport>,
    clippy: &Option<ClippyReport>,
) -> TimelineDeltaReport {
    // Index baseline by path (already relative from snapshot writer)
    let baseline_map: std::collections::HashMap<&str, &FileAnalysisSnapshot> = baseline
        .files
        .iter()
        .map(|f| (f.path.as_str(), f))
        .collect();

    let root_prefix = format!("{}/", root.trim_end_matches('/'));
    let mut by_file: std::collections::HashMap<String, FileDeltaEntry> =
        std::collections::HashMap::new();

    // PMAT by_path keys may be absolute — normalize to scan-root-relative
    if let Some(pmat) = pmat {
        for (raw_path, &idx) in &pmat.by_path {
            let rel_path = normalize_pmat_path(raw_path, &root_prefix);

            let baseline_entry = match baseline_map.get(rel_path) {
                Some(e) => e,
                None => continue,
            };

            let old_grade = baseline_entry.tdg_grade.as_deref().unwrap_or("");
            let new_grade = pmat.tdg.files[idx].grade.as_str();
            let tdg_grade_delta = grade_delta(old_grade, new_grade);

            let new_coverage = coverage
                .as_ref()
                .and_then(|c| c.by_path.get(rel_path))
                .map(|&ci| coverage.as_ref().unwrap().files[ci].summary.lines.percent);
            let coverage_pct_delta = match (baseline_entry.coverage_pct, new_coverage) {
                (Some(old), Some(new)) => Some(new - old),
                _ => None,
            };

            let new_clippy = clippy
                .as_ref()
                .and_then(|c| c.by_file.get(rel_path))
                .map(|d| d.total);
            let clippy_count_delta = match (baseline_entry.clippy_count, new_clippy) {
                (Some(old), Some(new)) => Some(new as i32 - old as i32),
                _ => None,
            };

            by_file.insert(
                rel_path.to_string(),
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
/// Scans the directory to collect snapshots. Use `prune_snapshots_from_list`
/// when a pre-collected list is available to avoid redundant I/O.
pub fn prune_snapshots(root: &str, max_count: usize) {
    let mut snapshots = list_snapshot_epochs(root);
    prune_snapshots_from_list(&mut snapshots, max_count);
}

/// Prune from a pre-collected, sorted list. Deletes oldest files first.
/// Silently ignores I/O errors (best-effort housekeeping).
fn prune_snapshots_from_list(snapshots: &mut Vec<(i64, PathBuf)>, max_count: usize) {
    if snapshots.len() <= max_count {
        return;
    }
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

        let report = compute_delta_report("/tmp/test", &baseline, &Some(pmat), &None, &None);
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

        let report = compute_delta_report("/tmp/test", &baseline, &Some(pmat), &None, &None);
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

        let report = compute_delta_report("/tmp/test", &baseline, &Some(pmat), &None, &None);
        assert!(
            !report.by_file.contains_key("src/new.rs"),
            "new file not in baseline should not produce a delta entry"
        );
    }

    #[test]
    fn test_compute_delta_baseline_epoch_set() {
        let baseline = make_snapshot_at(12345, "src/lib.rs", "B");
        let report = compute_delta_report("/tmp/test", &baseline, &None, &None, &None);
        assert_eq!(report.baseline_epoch, 12345);
    }

    #[test]
    fn test_normalize_pmat_path_strips_root() {
        assert_eq!(
            normalize_pmat_path("/Users/guy/project/src/main.rs", "/Users/guy/project/"),
            "src/main.rs"
        );
    }

    #[test]
    fn test_normalize_pmat_path_strips_dot_slash() {
        assert_eq!(normalize_pmat_path("./src/main.rs", "/other/root/"), "src/main.rs");
    }

    #[test]
    fn test_normalize_pmat_path_passthrough_relative() {
        assert_eq!(normalize_pmat_path("src/main.rs", "/other/root/"), "src/main.rs");
    }

    #[test]
    fn test_compute_delta_with_absolute_pmat_paths() {
        // Simulate PMAT returning absolute paths — the common production case
        let root = "/Users/guy/projects/sentrux/sentrux";
        let abs_path = format!("{}/sentrux-core/src/main.rs", root);

        // Build PMAT report with absolute path (as PMAT actually returns)
        let file = PmatFileScore {
            file_path: abs_path.clone(),
            grade: "A".to_string(),
            total: 80.0, structural_complexity: 80.0, semantic_complexity: 80.0,
            duplication_ratio: 80.0, coupling_score: 80.0, doc_coverage: 80.0,
            consistency_score: 80.0, entropy_score: 80.0, confidence: 0.9,
            language: "rust".to_string(), critical_defects_count: 0,
            has_critical_defects: false, penalties_applied: vec![],
        };
        let tdg = PmatTdgOutput {
            files: vec![file],
            average_score: 80.0, average_grade: "A".to_string(),
            total_files: 1, language_distribution: HashMap::new(),
        };
        let pmat = PmatReport::from_tdg(tdg, None);

        // Baseline snapshot uses relative path (as written by the fixed snapshot writer)
        let baseline = AnalysisSnapshot {
            computed_at: 100,
            commit_sha: String::new(),
            files: vec![FileAnalysisSnapshot {
                path: "sentrux-core/src/main.rs".to_string(),
                tdg_grade: Some("C".to_string()),
                coverage_pct: Some(50.0),
                clippy_count: Some(5),
            }],
        };

        let report = compute_delta_report(root, &baseline, &Some(pmat), &None, &None);
        assert!(
            report.by_file.contains_key("sentrux-core/src/main.rs"),
            "delta should match absolute PMAT path to relative baseline path after normalization; keys: {:?}",
            report.by_file.keys().collect::<Vec<_>>()
        );
        let entry = &report.by_file["sentrux-core/src/main.rs"];
        assert!(entry.tdg_grade_delta > 0, "C→A should be positive delta");
    }

    #[test]
    fn test_load_nearest_max_picks_highest_epoch() {
        let tmp = temp_root();
        let root = tmp.path().to_string_lossy().to_string();

        for &ep in &[100i64, 300, 200] {
            let snap = make_snapshot_at(ep, "src/foo.rs", "C");
            write_snapshot_at_epoch(tmp.path(), ep, &snap);
        }

        // load_nearest with i64::MAX acts as "load latest"
        let latest = load_nearest_snapshot(&root, i64::MAX)
            .expect("should find a snapshot");
        assert_eq!(latest.computed_at, 300, "should pick the highest epoch");
    }

    #[test]
    fn test_list_snapshot_epochs() {
        let tmp = temp_root();
        let root = tmp.path().to_string_lossy().to_string();

        for &ep in &[100i64, 300, 200] {
            let snap = make_snapshot_at(ep, "src/foo.rs", "C");
            write_snapshot_at_epoch(tmp.path(), ep, &snap);
        }

        let epochs = list_snapshot_epochs(&root);
        assert_eq!(epochs.len(), 3);
        let mut ep_values: Vec<i64> = epochs.iter().map(|(ep, _)| *ep).collect();
        ep_values.sort();
        assert_eq!(ep_values, vec![100, 200, 300]);
    }
}
