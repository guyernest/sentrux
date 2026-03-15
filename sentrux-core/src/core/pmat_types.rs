//! PMAT data types and grade helper functions.
//!
//! Provides deserialization types for PMAT JSON output (TDG and repo-score),
//! along with grade display and color-interpolation helpers.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

// ── PMAT TDG output types ────────────────────────────────────────────────

/// A single penalty applied to a file's PMAT score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatPenalty {
    pub source_metric: String,
    pub amount: f64,
    pub issue: String,
}

/// Per-file score entry from `pmat analyze tdg --format json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatFileScore {
    pub file_path: String,
    pub grade: String,
    pub total: f64,
    pub structural_complexity: f64,
    pub semantic_complexity: f64,
    pub duplication_ratio: f64,
    pub coupling_score: f64,
    pub doc_coverage: f64,
    pub consistency_score: f64,
    pub entropy_score: f64,
    pub confidence: f64,
    pub language: String,
    pub critical_defects_count: u32,
    pub has_critical_defects: bool,
    #[serde(default)]
    pub penalties_applied: Vec<PmatPenalty>,
}

/// Top-level output from `pmat analyze tdg --format json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatTdgOutput {
    pub files: Vec<PmatFileScore>,
    pub average_score: f64,
    pub average_grade: String,
    pub total_files: u32,
    #[serde(default)]
    pub language_distribution: HashMap<String, u32>,
}

// ── PMAT repo-score types ────────────────────────────────────────────────

/// A score category entry from `pmat repo-score --format json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatScoreCategory {
    pub score: f64,
    pub max_score: f64,
    pub percentage: f64,
    pub status: String,
}

/// Top-level output from `pmat repo-score --format json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatRepoScore {
    pub total_score: f64,
    pub grade: String,
    #[serde(default)]
    pub categories: std::collections::BTreeMap<String, PmatScoreCategory>,
    #[serde(default)]
    pub recommendations: Vec<serde_json::Value>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

// ── Aggregated report ────────────────────────────────────────────────────

/// Combined PMAT analysis report, with a normalized path index for O(1) lookup.
#[derive(Debug, Clone)]
pub struct PmatReport {
    pub tdg: PmatTdgOutput,
    pub repo_score: Option<PmatRepoScore>,
    /// Maps normalized file path (without "./" prefix) → index into `tdg.files`.
    pub by_path: HashMap<String, usize>,
}

impl PmatReport {
    /// Build a `PmatReport` from TDG output, constructing the normalized path index.
    ///
    /// Strips the "./" prefix from `file_path` so that lookups with bare paths like
    /// `"sentrux-core/src/main.rs"` match entries recorded as `"./sentrux-core/src/main.rs"`.
    pub fn from_tdg(tdg: PmatTdgOutput, repo_score: Option<PmatRepoScore>) -> PmatReport {
        let mut by_path = HashMap::with_capacity(tdg.files.len());
        for (i, f) in tdg.files.iter().enumerate() {
            let key = f.file_path.trim_start_matches("./").to_string();
            by_path.insert(key, i);
        }
        PmatReport { tdg, repo_score, by_path }
    }
}

// ── Grade helpers ────────────────────────────────────────────────────────

/// Map a PMAT grade string to a user-facing display string.
///
/// # Examples
/// ```
/// use sentrux_core::core::pmat_types::grade_to_display;
/// assert_eq!(grade_to_display("APLus"), "A+");
/// assert_eq!(grade_to_display("F"), "F");
/// assert_eq!(grade_to_display("unknown"), "?");
/// ```
pub fn grade_to_display(grade: &str) -> &'static str {
    match grade {
        "APLus"  => "A+",
        "A"      => "A",
        "AMinus" => "A-",
        "BPlus"  => "B+",
        "B"      => "B",
        "BMinus" => "B-",
        "CPlus"  => "C+",
        "C"      => "C",
        "CMinus" => "C-",
        "D"      => "D",
        "F"      => "F",
        _        => "?",
    }
}

/// Map a PMAT grade to a 0.0–1.0 value for color interpolation (1.0 = best, 0.0 = worst).
///
/// # Examples
/// ```
/// use sentrux_core::core::pmat_types::grade_to_t;
/// assert_eq!(grade_to_t("APLus"), 1.0_f32);
/// assert_eq!(grade_to_t("F"), 0.0_f32);
/// ```
pub fn grade_to_t(grade: &str) -> f32 {
    match grade {
        "APLus"  => 1.00,
        "A"      => 0.91,
        "AMinus" => 0.82,
        "BPlus"  => 0.73,
        "B"      => 0.64,
        "BMinus" => 0.55,
        "CPlus"  => 0.45,
        "C"      => 0.36,
        "CMinus" => 0.27,
        "D"      => 0.18,
        "F"      => 0.00,
        _        => 0.00,
    }
}

// ── Graph-metrics types ──────────────────────────────────────────────────

/// A single node entry from `pmat analyze graph-metrics --format json`.
/// Note: `name` is a bare filename (e.g. "channels.rs"), not a full path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetricsNode {
    pub name: String,
    pub degree_centrality: f64,
    pub betweenness_centrality: f64,
    pub closeness_centrality: f64,
    pub pagerank: f64,
    pub in_degree: u32,
    pub out_degree: u32,
}

/// Top-level output from `pmat analyze graph-metrics --format json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetricsOutput {
    pub nodes: Vec<GraphMetricsNode>,
    pub total_nodes: u32,
    pub total_edges: u32,
    pub density: f64,
    pub average_degree: f64,
    pub max_degree: u32,
    pub connected_components: u32,
}

/// Aggregated graph-metrics report with filename-based lookup index.
/// Not Serialize/Deserialize because HashMap key ordering is non-deterministic.
#[derive(Debug, Clone)]
pub struct GraphMetricsReport {
    pub data: GraphMetricsOutput,
    /// bare filename (e.g. "channels.rs") → index into `data.nodes`
    pub by_filename: HashMap<String, usize>,
}

impl GraphMetricsReport {
    /// Build a `GraphMetricsReport` from raw output, constructing the filename index.
    pub fn from_output(data: GraphMetricsOutput) -> Self {
        let mut by_filename = HashMap::with_capacity(data.nodes.len());
        for (i, n) in data.nodes.iter().enumerate() {
            by_filename.insert(n.name.clone(), i);
        }
        GraphMetricsReport { data, by_filename }
    }
}

// ── Coverage types ───────────────────────────────────────────────────────

/// A single coverage metric (lines, functions, etc.) from cargo-llvm-cov.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummaryMetric {
    pub count: u32,
    pub covered: u32,
    pub percent: f64,
}

/// Per-file summary from cargo-llvm-cov (lines + functions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverageSummary {
    pub lines: CoverageSummaryMetric,
    pub functions: CoverageSummaryMetric,
}

/// A single file entry from cargo-llvm-cov JSON output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageFileEntry {
    pub filename: String,
    pub summary: FileCoverageSummary,
}

/// A data section from cargo-llvm-cov JSON output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageDataSection {
    pub files: Vec<CoverageFileEntry>,
}

/// Top-level output from `cargo llvm-cov --json --summary-only`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageOutput {
    pub data: Vec<CoverageDataSection>,
}

/// Aggregated coverage report with scan-root-relative path lookup index.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub files: Vec<CoverageFileEntry>,
    /// scan-root-relative path → index into `files`
    pub by_path: HashMap<String, usize>,
    /// bare filename → index into `files` (for cross-index joins with graph-metrics)
    pub by_basename: HashMap<String, usize>,
}

impl CoverageReport {
    /// Build a `CoverageReport` from raw output, stripping the scan_root prefix
    /// from absolute filenames to produce scan-root-relative keys.
    ///
    /// Returns `None` if there is no data section.
    pub fn from_output(output: CoverageOutput, scan_root: &str) -> Option<Self> {
        let files = output.data.into_iter().next()?.files;
        let mut by_path = HashMap::with_capacity(files.len());
        let mut by_basename = HashMap::with_capacity(files.len());
        let root_with_sep = if scan_root.ends_with('/') {
            scan_root.to_string()
        } else {
            format!("{}/", scan_root)
        };
        for (i, f) in files.iter().enumerate() {
            if let Some(rel) = f.filename.strip_prefix(&root_with_sep) {
                by_path.insert(rel.to_string(), i);
                if let Some(base) = rel.rsplit('/').next() {
                    by_basename.insert(base.to_string(), i);
                }
            }
        }
        Some(CoverageReport { files, by_path, by_basename })
    }
}

// ── Clippy types ─────────────────────────────────────────────────────────

/// Per-file clippy warning data: total count + breakdown by category.
#[derive(Debug, Clone, Default)]
pub struct FileClippyData {
    pub total: u32,
    /// category ("style", "complexity", "correctness", "performance") → count
    pub by_category: HashMap<String, u32>,
}

/// Aggregated clippy report from `cargo clippy --message-format=json`.
#[derive(Debug, Clone)]
pub struct ClippyReport {
    /// scan-root-relative path → per-file warning data
    pub by_file: HashMap<String, FileClippyData>,
    /// bare filename → aggregated warning data (for cross-index joins with graph-metrics)
    pub by_basename: HashMap<String, FileClippyData>,
}

impl ClippyReport {
    /// Build a `ClippyReport` from a per-file map, constructing the basename index.
    pub fn from_by_file(by_file: HashMap<String, FileClippyData>) -> Self {
        let mut by_basename: HashMap<String, FileClippyData> = HashMap::new();
        for (path, data) in &by_file {
            if let Some(base) = path.rsplit('/').next() {
                let entry = by_basename.entry(base.to_string()).or_default();
                entry.total += data.total;
                for (cat, count) in &data.by_category {
                    *entry.by_category.entry(cat.clone()).or_insert(0) += count;
                }
            }
        }
        ClippyReport { by_file, by_basename }
    }
}

/// Map a clippy lint ID to a semantic category.
///
/// Categories: "correctness", "performance", "complexity", "style" (default).
/// Verified against 63 unique lint IDs found in the sentrux workspace.
pub fn lint_category(lint_id: &str) -> &'static str {
    match lint_id {
        // Correctness / suspicious numeric casts
        "clippy::while_float"
        | "clippy::unchecked_time_subtraction"
        | "clippy::cast_possible_truncation"
        | "clippy::cast_possible_wrap"
        | "clippy::cast_sign_loss"
        | "clippy::cast_precision_loss"
        | "clippy::cast_lossless" => "correctness",

        // Performance — unnecessary allocations and clones
        "clippy::implicit_clone"
        | "clippy::redundant_clone"
        | "clippy::needless_pass_by_value"
        | "clippy::needless_pass_by_ref_mut"
        | "clippy::map_unwrap_or"
        | "clippy::imprecise_flops"
        | "clippy::suboptimal_flops" => "performance",

        // Complexity — structural complexity
        "clippy::type_complexity"
        | "clippy::too_many_arguments"
        | "clippy::manual_let_else"
        | "clippy::useless_let_if_seq"
        | "clippy::map_entry"
        | "clippy::option_if_let_else"
        | "clippy::or_fun_call"
        | "clippy::manual_clamp"
        | "clippy::branches_sharing_code" => "complexity",

        // Everything else falls into style
        _ => "style",
    }
}

// ── Git diff overlay types ───────────────────────────────────────────────

/// Per-file data from a windowed git diff walk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffData {
    /// Number of commits that touched this file within the window
    pub commit_count: u32,
    /// Total lines added across all commits in the window
    pub lines_added: u32,
    /// Total lines removed across all commits in the window
    pub lines_removed: u32,
    /// True if this file was first created within the window
    pub is_new_file: bool,
}

impl FileDiffData {
    /// Combined change intensity: sqrt((lines_added + lines_removed) * commit_count).
    ///
    /// Uses geometric mean to combine line volume and commit frequency.
    /// Returns 0.0 for unchanged files (zero lines or zero commits).
    pub fn raw_intensity(&self) -> f64 {
        let lines = (self.lines_added + self.lines_removed) as f64;
        let commits = self.commit_count as f64;
        (lines * commits).sqrt()
    }
}

/// Aggregated git diff report for the current window, ready for color mapping.
#[derive(Debug, Clone)]
pub struct GitDiffReport {
    /// Per-file diff data, keyed by scan-root-relative path
    pub by_file: HashMap<String, FileDiffData>,
    /// Maximum raw_intensity across all files — used to normalize to 0..1.
    /// Defaults to 1.0 when no files changed (avoids division by zero).
    pub max_intensity: f64,
    /// The window this report was computed for
    pub window: crate::metrics::evo::git_walker::DiffWindow,
    /// Unix epoch when this report was computed
    pub computed_at: i64,
}

impl GitDiffReport {
    /// Build a `GitDiffReport` from windowed walk results.
    ///
    /// Aggregates per-commit records into per-file totals, marks new files,
    /// and computes the max_intensity for normalization.
    pub fn from_walk(
        records: Vec<crate::metrics::evo::git_walker::CommitRecord>,
        new_file_paths: HashSet<String>,
        window: crate::metrics::evo::git_walker::DiffWindow,
    ) -> Self {
        let mut by_file: HashMap<String, FileDiffData> = HashMap::new();
        for record in records {
            for file in record.files {
                let entry = by_file.entry(file.path).or_insert(FileDiffData {
                    commit_count: 0,
                    lines_added: 0,
                    lines_removed: 0,
                    is_new_file: false,
                });
                entry.commit_count += 1;
                entry.lines_added += file.added;
                entry.lines_removed += file.removed;
            }
        }
        // Mark new files
        for path in &new_file_paths {
            if let Some(entry) = by_file.get_mut(path) {
                entry.is_new_file = true;
            }
        }
        // Compute max_intensity
        let max_intensity = by_file
            .values()
            .map(|d| d.raw_intensity())
            .fold(0.0_f64, f64::max);
        let max_intensity = if max_intensity > 0.0 { max_intensity } else { 1.0 };
        let computed_at = crate::metrics::evo::git_walker::epoch_now();
        GitDiffReport { by_file, max_intensity, window, computed_at }
    }
}

/// Snapshot of per-file analysis scores at a point in time (for metric deltas).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisSnapshot {
    /// File path (scan-root-relative)
    pub path: String,
    /// PMAT TDG grade (e.g. "A", "B+")
    pub tdg_grade: Option<String>,
    /// Line coverage percentage (0.0–100.0)
    pub coverage_pct: Option<f64>,
    /// Clippy warning count
    pub clippy_count: Option<u32>,
}

/// Analysis snapshot for the entire project at a point in time.
///
/// Stored in `.sentrux/snapshot.json` and committed to git so historical
/// states can be retrieved for metric delta computation in GitDiff mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSnapshot {
    /// Unix epoch when this snapshot was taken
    pub computed_at: i64,
    /// Git commit SHA this snapshot corresponds to (may be empty if not committed)
    #[serde(default)]
    pub commit_sha: String,
    /// Per-file analysis scores
    pub files: Vec<FileAnalysisSnapshot>,
}

// ── GSD phase overlay types ──────────────────────────────────────────────

/// Status of a GSD planning phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStatus {
    /// Phase fully completed (ROADMAP.md checkbox is [x])
    Completed,
    /// First incomplete phase — currently in progress
    InProgress,
    /// Subsequent incomplete phases — not yet started
    Planned,
}

impl PhaseStatus {
    /// Human-readable label for this status.
    pub fn label(self) -> &'static str {
        match self {
            PhaseStatus::Completed => "Completed",
            PhaseStatus::InProgress => "In Progress",
            PhaseStatus::Planned => "Planned",
        }
    }
}

/// Metadata for a single GSD planning phase.
#[derive(Debug, Clone)]
pub struct PhaseInfo {
    /// Phase number string (e.g. "01", "02.1")
    pub number: String,
    /// Phase name (e.g. "Cleanup")
    pub name: String,
    /// Phase goal description
    pub goal: String,
    /// Completion status
    pub status: PhaseStatus,
    /// Completion date if available (from ROADMAP.md)
    pub completed_date: Option<String>,
    /// Files touched in this phase (scan-root-relative paths)
    pub files: Vec<String>,
    /// Commit range (from_sha, to_sha) detected from commit messages
    pub commit_range: Option<(String, String)>,
}

/// Aggregated GSD phase overlay report.
///
/// Built by parsing `.planning/ROADMAP.md`, `*-PLAN.md`, and `*-SUMMARY.md`
/// files at scan time. Used to color treemap files by which phase touched them.
#[derive(Debug, Clone)]
pub struct GsdPhaseReport {
    /// All phases in order
    pub phases: Vec<PhaseInfo>,
    /// Map from scan-root-relative file path → index into `phases`
    /// (most recent phase wins when a file appears in multiple phases)
    pub by_file: HashMap<String, usize>,
}

impl GsdPhaseReport {
    /// Total number of phases in this report.
    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }

    /// Look up which phase is associated with the given file path.
    ///
    /// Returns `None` if the file is not associated with any phase.
    pub fn phase_for_file(&self, path: &str) -> Option<&PhaseInfo> {
        self.by_file.get(path).map(|&idx| &self.phases[idx])
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod git_diff_tests {
    use super::*;

    #[test]
    fn file_diff_data_raw_intensity_changed_file() {
        let d = FileDiffData {
            commit_count: 4,
            lines_added: 100,
            lines_removed: 50,
            is_new_file: false,
        };
        let intensity = d.raw_intensity();
        assert!(intensity > 0.0, "raw_intensity should be > 0 for changed files, got {}", intensity);
        // sqrt((100+50) * 4) = sqrt(600) ≈ 24.49
        let expected = ((150.0_f64) * 4.0).sqrt();
        assert!((intensity - expected).abs() < 0.01, "expected {}, got {}", expected, intensity);
    }

    #[test]
    fn file_diff_data_raw_intensity_zero_lines() {
        let d = FileDiffData {
            commit_count: 0,
            lines_added: 0,
            lines_removed: 0,
            is_new_file: false,
        };
        assert_eq!(d.raw_intensity(), 0.0, "zero lines/commits = zero intensity");
    }

    #[test]
    fn git_diff_report_from_walk_max_intensity_positive() {
        use crate::metrics::evo::git_walker::{CommitRecord, CommitFile, DiffWindow};
        let records = vec![
            CommitRecord {
                author: "alice".to_string(),
                epoch: 1000,
                files: vec![
                    CommitFile { path: "src/foo.rs".to_string(), added: 10, removed: 5 },
                    CommitFile { path: "src/bar.rs".to_string(), added: 20, removed: 0 },
                ],
            },
        ];
        let new_files = std::collections::HashSet::new();
        let report = GitDiffReport::from_walk(records, new_files, DiffWindow::TimeSecs(86400));
        assert!(report.max_intensity > 0.0, "max_intensity should be > 0 when files changed");
        assert!(report.by_file.contains_key("src/foo.rs"));
        assert!(report.by_file.contains_key("src/bar.rs"));
    }

    #[test]
    fn git_diff_report_from_walk_empty_defaults_max_intensity_one() {
        use crate::metrics::evo::git_walker::DiffWindow;
        let report = GitDiffReport::from_walk(vec![], std::collections::HashSet::new(), DiffWindow::default());
        assert_eq!(report.max_intensity, 1.0, "empty walk should default max_intensity to 1.0");
    }

    #[test]
    fn git_diff_report_marks_new_files() {
        use crate::metrics::evo::git_walker::{CommitRecord, CommitFile, DiffWindow};
        let mut new_files = std::collections::HashSet::new();
        new_files.insert("src/new.rs".to_string());
        let records = vec![
            CommitRecord {
                author: "bob".to_string(),
                epoch: 2000,
                files: vec![CommitFile { path: "src/new.rs".to_string(), added: 50, removed: 0 }],
            },
        ];
        let report = GitDiffReport::from_walk(records, new_files, DiffWindow::CommitCount(1));
        let entry = report.by_file.get("src/new.rs").expect("new file should be in report");
        assert!(entry.is_new_file, "file in new_files set should have is_new_file=true");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TDG_FIXTURE: &str = r#"{
        "files": [
            {
                "file_path": "./sentrux-core/src/app/channels.rs",
                "grade": "APLus",
                "total": 97.5,
                "structural_complexity": 95.0,
                "semantic_complexity": 98.0,
                "duplication_ratio": 100.0,
                "coupling_score": 95.0,
                "doc_coverage": 90.0,
                "consistency_score": 100.0,
                "entropy_score": 98.0,
                "confidence": 0.85,
                "language": "rust",
                "critical_defects_count": 0,
                "has_critical_defects": false,
                "penalties_applied": []
            }
        ],
        "average_score": 97.5,
        "average_grade": "APLus",
        "total_files": 1,
        "language_distribution": {"rust": 1}
    }"#;

    const REPO_SCORE_FIXTURE: &str = r#"{
        "total_score": 85.0,
        "grade": "B",
        "categories": {
            "quality": {
                "score": 42.5,
                "max_score": 50.0,
                "percentage": 85.0,
                "status": "Good"
            }
        },
        "recommendations": [],
        "metadata": null
    }"#;

    // ── grade_to_display ────────────────────────────────────────────────

    #[test]
    fn grade_to_display_aplus() {
        assert_eq!(grade_to_display("APLus"), "A+");
    }

    #[test]
    fn grade_to_display_all_11_variants() {
        assert_eq!(grade_to_display("APLus"),  "A+");
        assert_eq!(grade_to_display("A"),      "A");
        assert_eq!(grade_to_display("AMinus"), "A-");
        assert_eq!(grade_to_display("BPlus"),  "B+");
        assert_eq!(grade_to_display("B"),      "B");
        assert_eq!(grade_to_display("BMinus"), "B-");
        assert_eq!(grade_to_display("CPlus"),  "C+");
        assert_eq!(grade_to_display("C"),      "C");
        assert_eq!(grade_to_display("CMinus"), "C-");
        assert_eq!(grade_to_display("D"),      "D");
        assert_eq!(grade_to_display("F"),      "F");
    }

    #[test]
    fn grade_to_display_unknown_returns_question_mark() {
        assert_eq!(grade_to_display("unknown"), "?");
        assert_eq!(grade_to_display(""), "?");
        assert_eq!(grade_to_display("Z"), "?");
    }

    // ── grade_to_t ──────────────────────────────────────────────────────

    #[test]
    fn grade_to_t_aplus_is_1_0() {
        assert_eq!(grade_to_t("APLus"), 1.0_f32);
    }

    #[test]
    fn grade_to_t_f_is_0_0() {
        assert_eq!(grade_to_t("F"), 0.0_f32);
    }

    #[test]
    fn grade_to_t_all_in_range() {
        let grades = ["APLus", "A", "AMinus", "BPlus", "B", "BMinus", "CPlus", "C", "CMinus", "D", "F"];
        for g in &grades {
            let t = grade_to_t(g);
            assert!((0.0..=1.0).contains(&t), "grade_to_t({}) = {} out of range", g, t);
        }
    }

    #[test]
    fn grade_to_t_ordered() {
        // Higher grades must have higher t values.
        let grades = ["APLus", "A", "AMinus", "BPlus", "B", "BMinus", "CPlus", "C", "CMinus", "D", "F"];
        let ts: Vec<f32> = grades.iter().map(|g| grade_to_t(g)).collect();
        for i in 1..ts.len() {
            assert!(ts[i - 1] > ts[i], "grade_to_t not strictly decreasing: {} ({}) >= {} ({})",
                grades[i-1], ts[i-1], grades[i], ts[i]);
        }
    }

    // ── PmatTdgOutput deserialization ────────────────────────────────────

    #[test]
    fn tdg_output_deserializes() {
        let output: PmatTdgOutput = serde_json::from_str(TDG_FIXTURE).expect("TDG fixture should deserialize");
        assert_eq!(output.total_files, 1);
        assert_eq!(output.average_grade, "APLus");
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].grade, "APLus");
        assert_eq!(output.files[0].file_path, "./sentrux-core/src/app/channels.rs");
        assert!(!output.files[0].has_critical_defects);
    }

    // ── PmatRepoScore deserialization ────────────────────────────────────

    #[test]
    fn repo_score_deserializes() {
        let score: PmatRepoScore = serde_json::from_str(REPO_SCORE_FIXTURE).expect("RepoScore fixture should deserialize");
        assert_eq!(score.grade, "B");
        assert!((score.total_score - 85.0).abs() < 0.01);
        assert!(score.categories.contains_key("quality"));
    }

    // ── PmatReport::from_tdg ─────────────────────────────────────────────

    #[test]
    fn pmat_report_strips_dot_slash_prefix() {
        let tdg: PmatTdgOutput = serde_json::from_str(TDG_FIXTURE).expect("fixture should parse");
        let report = PmatReport::from_tdg(tdg, None);
        // The file_path is "./sentrux-core/src/app/channels.rs" — by_path should strip "./"
        assert!(report.by_path.contains_key("sentrux-core/src/app/channels.rs"),
            "by_path should have key without './' prefix, got: {:?}", report.by_path.keys().collect::<Vec<_>>());
        // Should NOT contain the raw "./..." key
        assert!(!report.by_path.contains_key("./sentrux-core/src/app/channels.rs"),
            "by_path should not have key with './' prefix");
    }

    #[test]
    fn pmat_report_lookup_by_index() {
        let tdg: PmatTdgOutput = serde_json::from_str(TDG_FIXTURE).expect("fixture should parse");
        let report = PmatReport::from_tdg(tdg, None);
        let idx = report.by_path["sentrux-core/src/app/channels.rs"];
        assert_eq!(report.tdg.files[idx].grade, "APLus");
    }

    // ── Graph-metrics types ──────────────────────────────────────────────

    const GRAPH_METRICS_FIXTURE: &str = r#"{
        "nodes": [
            {
                "name": "channels.rs",
                "degree_centrality": 0.057,
                "betweenness_centrality": 0.0,
                "closeness_centrality": 0.0,
                "pagerank": 0.01136,
                "in_degree": 4,
                "out_degree": 1
            },
            {
                "name": "state.rs",
                "degree_centrality": 0.045,
                "betweenness_centrality": 0.01,
                "closeness_centrality": 0.02,
                "pagerank": 0.00980,
                "in_degree": 3,
                "out_degree": 2
            }
        ],
        "total_nodes": 88,
        "total_edges": 105,
        "density": 0.027,
        "average_degree": 2.39,
        "max_degree": 75,
        "connected_components": 24
    }"#;

    #[test]
    fn graph_metrics_output_deserializes() {
        let output: GraphMetricsOutput =
            serde_json::from_str(GRAPH_METRICS_FIXTURE).expect("graph-metrics fixture should deserialize");
        assert_eq!(output.nodes.len(), 2);
        assert_eq!(output.total_nodes, 88);
        assert_eq!(output.total_edges, 105);
        assert_eq!(output.nodes[0].name, "channels.rs");
        assert!((output.nodes[0].pagerank - 0.01136).abs() < 1e-6);
        assert_eq!(output.nodes[0].in_degree, 4);
    }

    #[test]
    fn graph_metrics_report_by_filename() {
        let output: GraphMetricsOutput =
            serde_json::from_str(GRAPH_METRICS_FIXTURE).expect("fixture should parse");
        let report = GraphMetricsReport::from_output(output);
        assert!(report.by_filename.contains_key("channels.rs"),
            "by_filename should contain 'channels.rs'");
        assert!(report.by_filename.contains_key("state.rs"),
            "by_filename should contain 'state.rs'");
        let idx = report.by_filename["channels.rs"];
        assert_eq!(report.data.nodes[idx].pagerank, 0.01136);
    }

    // ── Coverage types ───────────────────────────────────────────────────

    const COVERAGE_FIXTURE: &str = r#"{
        "data": [
            {
                "files": [
                    {
                        "filename": "/Users/guy/projects/sentrux/sentrux/sentrux-core/src/app/channels.rs",
                        "summary": {
                            "lines": {"count": 100, "covered": 85, "percent": 85.0},
                            "functions": {"count": 10, "covered": 8, "percent": 80.0}
                        }
                    }
                ]
            }
        ]
    }"#;

    #[test]
    fn coverage_output_deserializes() {
        let output: CoverageOutput =
            serde_json::from_str(COVERAGE_FIXTURE).expect("coverage fixture should deserialize");
        assert_eq!(output.data.len(), 1);
        assert_eq!(output.data[0].files.len(), 1);
        assert!((output.data[0].files[0].summary.lines.percent - 85.0).abs() < 0.01);
    }

    #[test]
    fn coverage_report_path_normalization() {
        let output: CoverageOutput =
            serde_json::from_str(COVERAGE_FIXTURE).expect("fixture should parse");
        let scan_root = "/Users/guy/projects/sentrux/sentrux";
        let report = CoverageReport::from_output(output, scan_root)
            .expect("should produce a CoverageReport");
        let key = "sentrux-core/src/app/channels.rs";
        assert!(report.by_path.contains_key(key),
            "by_path should have relative key '{}', got: {:?}",
            key, report.by_path.keys().collect::<Vec<_>>());
        let idx = report.by_path[key];
        assert!((report.files[idx].summary.lines.percent - 85.0).abs() < 0.01);
    }

    // ── Clippy types ─────────────────────────────────────────────────────

    const CLIPPY_NDJSON_FIXTURE: &str = concat!(
        r#"{"reason":"compiler-artifact","package_id":"sentrux-core 0.1.0"}"#, "\n",
        r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::doc_markdown","explanation":null},"spans":[{"file_name":"sentrux-core/src/analysis/parser/strings.rs","is_primary":true}],"children":[]}}"#, "\n",
        r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::cast_possible_truncation","explanation":null},"spans":[{"file_name":"sentrux-core/src/analysis/parser/strings.rs","is_primary":true}],"children":[]}}"#, "\n",
        r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::implicit_clone","explanation":null},"spans":[{"file_name":"sentrux-core/src/renderer/colors.rs","is_primary":true}],"children":[]}}"#, "\n",
        r#"{"reason":"compiler-message","message":{"level":"error","code":{"code":"E0308","explanation":null},"spans":[{"file_name":"sentrux-core/src/lib.rs","is_primary":true}],"children":[]}}"#, "\n"
    );

    fn build_clippy_report_from_fixture() -> ClippyReport {
        let mut by_file: HashMap<String, FileClippyData> = HashMap::new();
        for line in CLIPPY_NDJSON_FIXTURE.lines() {
            if line.is_empty() {
                continue;
            }
            let obj: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if obj["reason"] != "compiler-message" {
                continue;
            }
            let msg = &obj["message"];
            if msg["level"] != "warning" {
                continue;
            }
            let lint_id = match msg["code"]["code"].as_str() {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            };
            let spans = match msg["spans"].as_array() {
                Some(s) => s,
                None => continue,
            };
            for span in spans {
                if span["is_primary"].as_bool() != Some(true) {
                    continue;
                }
                let fname = match span["file_name"].as_str() {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => continue,
                };
                let entry = by_file.entry(fname).or_default();
                entry.total += 1;
                *entry.by_category.entry(lint_category(&lint_id).to_string()).or_insert(0) += 1;
            }
        }
        ClippyReport::from_by_file(by_file)
    }

    #[test]
    fn clippy_report_from_ndjson() {
        let report = build_clippy_report_from_fixture();
        // strings.rs should have 2 warnings (doc_markdown=style + cast_possible_truncation=correctness)
        let strings_data = report.by_file.get("sentrux-core/src/analysis/parser/strings.rs")
            .expect("strings.rs should be in report");
        assert_eq!(strings_data.total, 2, "strings.rs should have 2 warnings");
        assert_eq!(strings_data.by_category.get("style"), Some(&1));
        assert_eq!(strings_data.by_category.get("correctness"), Some(&1));
        // colors.rs should have 1 warning (implicit_clone=performance)
        let colors_data = report.by_file.get("sentrux-core/src/renderer/colors.rs")
            .expect("colors.rs should be in report");
        assert_eq!(colors_data.total, 1);
        assert_eq!(colors_data.by_category.get("performance"), Some(&1));
        // lib.rs should NOT be in report (it was an error, not a warning)
        assert!(!report.by_file.contains_key("sentrux-core/src/lib.rs"),
            "error-level diagnostics should not be in clippy report");
    }

    #[test]
    fn lint_category_mapping() {
        // Correctness
        assert_eq!(lint_category("clippy::cast_possible_truncation"), "correctness");
        assert_eq!(lint_category("clippy::cast_sign_loss"), "correctness");
        // Performance
        assert_eq!(lint_category("clippy::implicit_clone"), "performance");
        assert_eq!(lint_category("clippy::needless_pass_by_value"), "performance");
        // Complexity
        assert_eq!(lint_category("clippy::type_complexity"), "complexity");
        assert_eq!(lint_category("clippy::too_many_arguments"), "complexity");
        // Style (default)
        assert_eq!(lint_category("clippy::doc_markdown"), "style");
        assert_eq!(lint_category("clippy::some_unknown_lint"), "style");
    }
}
