//! PMAT data types and grade helper functions.
//!
//! Provides deserialization types for PMAT JSON output (TDG and repo-score),
//! along with grade display and color-interpolation helpers.

use std::collections::HashMap;
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

// ── Tests ────────────────────────────────────────────────────────────────

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
