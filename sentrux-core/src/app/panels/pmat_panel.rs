//! PMAT health panel — shows TDG repo health summary and per-file TDG breakdown.
//!
//! Serves two purposes:
//! - **Health summary**: repo-score grade, TDG average grade/score, file count
//! - **File detail**: TDG component scores for the currently selected file

use egui::Ui;
use crate::app::state::AppState;
use crate::core::pmat_types::{grade_to_display, PmatReport};

/// Draw the PMAT analysis section inside the metrics panel.
///
/// When no pmat_report is available, shows a graceful fallback message.
/// When available, shows health summary and (if a file is selected) TDG breakdown.
pub fn draw_pmat_panel(ui: &mut Ui, state: &AppState) {
    let Some(report) = &state.pmat_report else {
        ui.label(
            egui::RichText::new("PMAT analysis not available")
                .monospace()
                .size(9.0)
                .color(ui.visuals().weak_text_color()),
        );
        ui.label(
            egui::RichText::new("Install: cargo install pmat")
                .monospace()
                .size(9.0)
                .color(ui.visuals().weak_text_color()),
        );
        return;
    };

    // --- Health Summary ---
    draw_health_summary(ui, report);

    ui.separator();

    // --- File Detail (if file selected) ---
    if let Some(selected) = &state.selected_path {
        draw_file_detail(ui, report, selected);
    } else {
        ui.label(
            egui::RichText::new("Select a file to see TDG breakdown")
                .monospace()
                .size(9.0)
                .color(ui.visuals().weak_text_color()),
        );
    }
}

/// Draw the project health summary section.
fn draw_health_summary(ui: &mut Ui, report: &PmatReport) {
    ui.label(
        egui::RichText::new("PMAT Health")
            .monospace()
            .size(10.0)
            .strong(),
    );

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("TDG Grade:").monospace().size(9.0));
        ui.label(
            egui::RichText::new(grade_to_display(&report.tdg.average_grade))
                .monospace()
                .size(9.0)
                .strong(),
        );
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("TDG Score:").monospace().size(9.0));
        ui.label(
            egui::RichText::new(format!("{:.1}", report.tdg.average_score))
                .monospace()
                .size(9.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Files:").monospace().size(9.0));
        ui.label(
            egui::RichText::new(format!("{}", report.tdg.total_files))
                .monospace()
                .size(9.0),
        );
    });

    if let Some(repo) = &report.repo_score {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Repo Score:").monospace().size(9.0));
            ui.label(
                egui::RichText::new(format!(
                    "{} ({:.0}/110)",
                    grade_to_display(&repo.grade),
                    repo.total_score
                ))
                .monospace()
                .size(9.0)
                .strong(),
            );
        });

        egui::CollapsingHeader::new(
            egui::RichText::new("Categories").monospace().size(9.0),
        )
        .id_source("pmat_categories")
        .show(ui, |ui| {
            let mut categories: Vec<(&String, &crate::core::pmat_types::PmatScoreCategory)> =
                repo.categories.iter().collect();
            categories.sort_by_key(|(name, _)| name.as_str());
            for (name, cat) in categories {
                let status_color = match cat.status.as_str() {
                    "Pass" => egui::Color32::from_rgb(72, 191, 145),
                    "Warning" => egui::Color32::from_rgb(255, 193, 7),
                    _ => egui::Color32::from_rgb(244, 67, 54),
                };
                ui.horizontal(|ui| {
                    ui.colored_label(status_color, egui::RichText::new("●").size(9.0));
                    ui.label(
                        egui::RichText::new(format!("{}: {:.0}/{:.0}", name, cat.score, cat.max_score))
                            .monospace()
                            .size(9.0),
                    );
                });
            }
        });
    }
}

/// Draw the TDG breakdown for the currently selected file.
fn draw_file_detail(ui: &mut Ui, report: &PmatReport, path: &str) {
    let file_score = report
        .by_path
        .get(path)
        .and_then(|&idx| report.tdg.files.get(idx));

    let Some(score) = file_score else {
        ui.label(
            egui::RichText::new("No TDG data for this file")
                .monospace()
                .size(9.0)
                .color(ui.visuals().weak_text_color()),
        );
        return;
    };

    ui.label(
        egui::RichText::new("TDG Breakdown")
            .monospace()
            .size(10.0)
            .strong(),
    );
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Grade:").monospace().size(9.0));
        ui.label(
            egui::RichText::new(format!(
                "{} ({:.1}/100)",
                grade_to_display(&score.grade),
                score.total
            ))
            .monospace()
            .size(9.0)
            .strong(),
        );
    });

    egui::CollapsingHeader::new(
        egui::RichText::new("Component Scores").monospace().size(9.0),
    )
    .id_source("pmat_components")
    .default_open(true)
    .show(ui, |ui| {
        score_row(ui, "Structural", score.structural_complexity);
        score_row(ui, "Semantic", score.semantic_complexity);
        score_row(ui, "Duplication", score.duplication_ratio);
        score_row(ui, "Coupling", score.coupling_score);
        score_row(ui, "Doc Coverage", score.doc_coverage);
        score_row(ui, "Consistency", score.consistency_score);
        score_row(ui, "Entropy", score.entropy_score);
    });

    if !score.penalties_applied.is_empty() {
        egui::CollapsingHeader::new(
            egui::RichText::new("Penalties").monospace().size(9.0),
        )
        .id_source("pmat_penalties")
        .show(ui, |ui| {
            for p in &score.penalties_applied {
                ui.label(
                    egui::RichText::new(format!(
                        "-{:.1}: {} ({})",
                        p.amount, p.issue, p.source_metric
                    ))
                    .monospace()
                    .size(9.0),
                );
            }
        });
    }

    if score.has_critical_defects {
        ui.colored_label(
            egui::Color32::from_rgb(244, 67, 54),
            egui::RichText::new(format!("{} critical defect(s)", score.critical_defects_count))
                .monospace()
                .size(9.0),
        );
    }
}

/// Render a single component score row: "Label: value".
fn score_row(ui: &mut Ui, label: &str, value: f64) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{label}:")).monospace().size(9.0));
        ui.label(egui::RichText::new(format!("{value:.1}")).monospace().size(9.0));
    });
}

/// Helper: whether a badge/detail should be drawn based on size threshold.
/// Extracted for testability.
pub fn should_show_detail(pmat_report: Option<&PmatReport>, selected_path: Option<&str>) -> bool {
    pmat_report.is_some() && selected_path.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pmat_types::{PmatTdgOutput, PmatFileScore, PmatReport};
    use std::collections::HashMap;

    fn make_test_file_score(path: &str, grade: &str, total: f64) -> PmatFileScore {
        PmatFileScore {
            file_path: path.to_string(),
            grade: grade.to_string(),
            total,
            structural_complexity: total,
            semantic_complexity: total,
            duplication_ratio: 100.0,
            coupling_score: total,
            doc_coverage: total,
            consistency_score: total,
            entropy_score: total,
            confidence: 0.9,
            language: "rust".to_string(),
            critical_defects_count: 0,
            has_critical_defects: false,
            penalties_applied: vec![],
        }
    }

    fn make_test_report(paths: &[(&str, &str, f64)]) -> PmatReport {
        let files = paths
            .iter()
            .map(|(p, g, t)| make_test_file_score(p, g, *t))
            .collect();
        let tdg = PmatTdgOutput {
            files,
            average_score: 80.0,
            average_grade: "B".to_string(),
            total_files: paths.len() as u32,
            language_distribution: HashMap::new(),
        };
        PmatReport::from_tdg(tdg, None)
    }

    /// Panel graceful fallback: file lookup on missing path returns None without panic.
    #[test]
    fn file_detail_missing_path_returns_none_gracefully() {
        let report = make_test_report(&[("./src/main.rs", "APLus", 97.5)]);
        // Lookup a path that is NOT in the report — should return None, not panic
        let result = report.by_path.get("nonexistent/path.rs");
        assert!(result.is_none(), "Missing path should yield None, not panic");
    }

    /// Panel contract: by_path lookup works for bare paths (no "./" prefix).
    #[test]
    fn file_detail_by_path_lookup_strips_dot_slash() {
        let report = make_test_report(&[("./src/main.rs", "A", 91.0)]);
        let idx = report.by_path.get("src/main.rs");
        assert!(idx.is_some(), "Should find 'src/main.rs' after stripping './'");
        let idx = idx.unwrap();
        assert_eq!(report.tdg.files[*idx].grade, "A");
        assert!((report.tdg.files[*idx].total - 91.0).abs() < 0.01);
    }

    /// Panel contract: with pmat_report=None and selected_path=None, should_show_detail is false.
    #[test]
    fn should_show_detail_none_report_is_false() {
        assert!(!should_show_detail(None, None));
        assert!(!should_show_detail(None, Some("src/main.rs")));
    }

    /// Panel contract: with both pmat_report and selected_path set, should_show_detail is true.
    #[test]
    fn should_show_detail_with_report_and_path_is_true() {
        let report = make_test_report(&[("./src/main.rs", "B", 70.0)]);
        assert!(should_show_detail(Some(&report), Some("src/main.rs")));
    }

    /// Panel render contract: report with no matching path for selected file returns gracefully.
    #[test]
    fn file_detail_no_match_for_selected_path() {
        let report = make_test_report(&[("./src/lib.rs", "C", 55.0)]);
        // selected path exists in snapshot but not in PMAT report — graceful None
        let result = report.by_path.get("src/main.rs").and_then(|&idx| report.tdg.files.get(idx));
        assert!(result.is_none(), "Should be None when selected file not in PMAT report");
    }
}
