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
    pub categories: HashMap<String, PmatScoreCategory>,
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
}
