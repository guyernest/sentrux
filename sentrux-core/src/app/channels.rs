//! Inter-thread communication types for scanner and layout workers.
//!
//! All communication between the main UI thread and background workers
//! (scanner, layout) goes through typed channels using these messages.
//! Each message carries a generation counter for stale-result rejection.

use crate::layout::types::{LayoutMode, RenderData, ScaleMode, SizeMode};
use crate::core::settings::Settings;
use crate::layout::types::FocusMode;
use crate::core::snapshot::{ScanProgress, Snapshot};
use crate::metrics::evo::EvolutionReport;
use crate::metrics::testgap::TestGapReport;
use crate::core::pmat_types::{PmatReport, GraphMetricsReport, CoverageReport, ClippyReport, GsdPhaseReport};
use std::collections::HashSet;
use std::sync::Arc;

/// Commands sent to the scanner thread.
/// Each command carries its own `gen` (generation counter) so the scanner
/// doesn't need a shared atomic — eliminates the send/bump race where
/// the scanner could load a stale generation between try_send and fetch_add
/// on the main thread. [ref:13696c9c]
pub enum ScanCommand {
    /// Full scan of a directory — walks filesystem, parses all files, builds graphs.
    FullScan {
        /// Absolute path to the directory to scan
        root: String,
        /// Resource limits for scanning and parsing
        limits: crate::app::scan_threads::ScanLimits,
        /// Generation counter for stale-result rejection
        gen: u64,
    },
    /// Incremental rescan — re-parses only changed files, patches existing snapshot.
    Rescan {
        /// Absolute path to the directory root
        root: String,
        /// Relative paths of files that changed (from watcher)
        changed: Vec<String>,
        /// Previous snapshot to patch (graph rebuild uses old data for unchanged files)
        old_snap: Arc<Snapshot>,
        /// Resource limits for scanning and parsing
        limits: crate::app::scan_threads::ScanLimits,
        /// Generation counter for stale-result rejection
        gen: u64,
    },
}

/// All reports computed on the scanner thread after scan completion.
/// Bundled into a struct to keep ScanMsg::Complete tidy as we add more analyses.
pub struct ScanReports {
    /// Git evolution report (churn, temporal coupling, bus factor)
    pub evolution: Option<EvolutionReport>,
    /// Test gap analysis (untested high-risk files)
    pub test_gaps: Option<TestGapReport>,
    /// PMAT TDG + repo-score analysis — None if PMAT subprocess fails
    pub pmat: Option<PmatReport>,
    /// PMAT graph-metrics report (PageRank, centrality) — None if subprocess fails
    pub graph_metrics: Option<GraphMetricsReport>,
    /// Cargo clippy warnings grouped by file — None if subprocess fails
    pub clippy: Option<ClippyReport>,
}

/// Messages from scanner thread → main thread.
/// TreeReady and Complete carry a generation counter so the main thread
/// can reject stale results from a previous scan (e.g., after rapid
/// directory switches). [ref:93cf32d4]
pub enum ScanMsg {
    /// Scan progress update (step name + percentage)
    Progress(ScanProgress),
    /// File tree ready (before graphs) — enables early rendering
    TreeReady(Arc<Snapshot>, u64),
    /// Scan fully complete with all analysis reports
    Complete(Arc<Snapshot>, u64, Box<ScanReports>),
    /// Scan failed with error message
    Error(String, u64),
    /// On-demand coverage run completed — carries the report to store on AppState
    CoverageReady(CoverageReport),
    /// On-demand coverage run failed — error message for logging
    CoverageError(String),
    /// On-demand git diff analysis completed — carries the report to store on AppState
    GitDiffReady(crate::core::pmat_types::GitDiffReport),
    /// On-demand git diff analysis failed — error message for logging
    GitDiffError(String),
    /// GSD phase overlay analysis completed — carries the report to store on AppState
    GsdPhaseReady(GsdPhaseReport),
    /// GSD phase overlay analysis failed — error message for logging
    GsdPhaseError(String),
}

/// Messages from main thread → layout thread.
/// Contains all data the layout engine needs to produce RenderData.
pub struct LayoutRequest {
    /// Current scan snapshot (shared, not cloned)
    pub snapshot: Arc<Snapshot>,
    /// What metric determines file block area
    pub size_mode: SizeMode,
    /// Scaling transform for size compression
    pub scale_mode: ScaleMode,
    /// Spatial arrangement algorithm
    pub layout_mode: LayoutMode,
    /// Available viewport width in screen pixels
    pub viewport_w: f64,
    /// Available viewport height in screen pixels
    pub viewport_h: f64,
    /// Current drill-down path prefix (empty = show everything)
    pub drill_path: Option<String>,
    /// Layout version at time of request — returned in LayoutMsg::Ready for matching.
    pub version: u64,
    /// BUG 1 fix: snapshot of heat values from HeatTracker for SizeMode::Heat layout.
    /// None when heat mode is not active (avoids cloning HashMap every frame).
    pub heat_map: Option<std::collections::HashMap<String, f64>>,
    /// User-tunable settings (cloned per request so layout thread reads consistent values)
    pub settings: Settings,
    /// Focus mode filter — controls which files are visible in layout
    pub focus_mode: FocusMode,
    /// Entry-point file paths for FocusMode::EntryPoints filtering.
    /// Wrapped in Arc so cloning from AppState is O(1) atomic increment.
    pub entry_point_files: Arc<HashSet<String>>,
    /// User-hidden paths (files or directory prefixes) — weight 0 in layout.
    /// Wrapped in Arc so cloning from AppState is O(1) atomic increment.
    pub hidden_paths: Arc<HashSet<String>>,
    /// Pre-computed impact files for ImpactRadius focus mode (transitive dependents).
    /// Wrapped in Arc so cloning from AppState is O(1) atomic increment.
    pub impact_files: Option<Arc<HashSet<String>>>,
    /// External per-file weights for analysis-backed SizeModes (PageRank, Centrality, ClippyCount).
    /// Built from analysis reports, keyed by file path.
    pub external_weights: Option<std::collections::HashMap<String, f64>>,
}

/// Messages from the layout thread to the main UI thread.
pub enum LayoutMsg {
    /// Layout computation complete: render data + version for stale-result rejection
    Ready(RenderData, u64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pmat_types::{PmatTdgOutput, PmatFileScore, PmatReport};
    use std::collections::HashMap;

    fn make_tdg_with_files(paths: &[&str]) -> PmatTdgOutput {
        let files = paths.iter().map(|p| PmatFileScore {
            file_path: p.to_string(),
            grade: "B".to_string(),
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
        }).collect();
        PmatTdgOutput {
            files,
            average_score: 70.0,
            average_grade: "B".to_string(),
            total_files: paths.len() as u32,
            language_distribution: HashMap::new(),
        }
    }

    /// Scan pipeline contract: PmatReport::from_tdg strips "./" prefix,
    /// enabling O(1) lookup by bare path (as stored in snapshot file paths).
    #[test]
    fn pmat_report_pipeline_by_path_lookup() {
        let tdg = make_tdg_with_files(&[
            "./src/main.rs",
            "./src/lib.rs",
        ]);
        let report = PmatReport::from_tdg(tdg, None);

        // Bare path lookup (no "./" prefix) — what scan pipeline uses
        let idx = report.by_path.get("src/main.rs");
        assert!(idx.is_some(), "by_path should find 'src/main.rs' (stripped from './src/main.rs')");
        let idx = idx.unwrap();
        assert_eq!(report.tdg.files[*idx].file_path, "./src/main.rs");

        // Lookup of second file
        let idx2 = report.by_path.get("src/lib.rs").expect("src/lib.rs should be in by_path");
        assert_eq!(report.tdg.files[*idx2].file_path, "./src/lib.rs");
    }

    /// ScanMsg has GitDiffReady and GitDiffError variants
    #[test]
    fn scan_msg_has_git_diff_variants() {
        use crate::core::pmat_types::GitDiffReport;
        use crate::metrics::evo::git_walker::DiffWindow;
        use std::collections::HashMap;
        let report = GitDiffReport {
            by_file: HashMap::new(),
            max_intensity: 1.0,
            window: DiffWindow::TimeSecs(86400),
            computed_at: 0,
        };
        let msg = ScanMsg::GitDiffReady(report);
        assert!(matches!(msg, ScanMsg::GitDiffReady(_)));

        let err_msg = ScanMsg::GitDiffError("test error".to_string());
        assert!(matches!(err_msg, ScanMsg::GitDiffError(_)));
    }

    /// Scan pipeline contract: ScanReports can carry a PmatReport and new graph/clippy fields.
    #[test]
    fn scan_reports_has_pmat_field() {
        let tdg = make_tdg_with_files(&["./src/main.rs"]);
        let report = PmatReport::from_tdg(tdg, None);
        let reports = ScanReports {
            evolution: None,
            test_gaps: None,
            pmat: Some(report),
            graph_metrics: None,
            clippy: None,
        };
        assert!(reports.pmat.is_some());
        assert_eq!(reports.pmat.as_ref().unwrap().tdg.total_files, 1);
        assert!(reports.graph_metrics.is_none());
        assert!(reports.clippy.is_none());
    }
}
