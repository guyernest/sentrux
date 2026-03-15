//! PMAT health panel — shows TDG repo health summary and per-file TDG breakdown.
//!
//! Serves two purposes:
//! - **Health summary**: repo-score grade, TDG average grade/score, file count
//! - **File detail**: TDG component scores for the currently selected file

use egui::Ui;
use crate::app::state::AppState;
use crate::core::pmat_types::{grade_to_display, PmatReport};
use crate::layout::types::ColorMode;

/// Draw the PMAT analysis section inside the metrics panel.
///
/// When no pmat_report is available, shows a graceful fallback message.
/// When available, shows health summary and (if a file is selected) TDG breakdown,
/// plus Code Rank, Test Coverage, and Clippy Analysis sections.
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
        ui.separator();
        draw_rank_section(ui, state, selected);
        ui.separator();
        draw_coverage_section(ui, state, selected);
        ui.separator();
        draw_clippy_section(ui, state, selected);
        // Show git diff changes section when in GitDiff mode
        if state.color_mode == ColorMode::GitDiff {
            ui.separator();
            draw_git_diff_section(ui, state, selected);
        }
        // Show GSD phase association when in GsdPhase mode
        if state.color_mode == ColorMode::GsdPhase {
            ui.separator();
            draw_gsd_phase_section(ui, state, selected);
        }
    } else {
        ui.label(
            egui::RichText::new("Select a file to see TDG breakdown")
                .monospace()
                .size(9.0)
                .color(ui.visuals().weak_text_color()),
        );
    }
}

/// Draw the Code Rank (PageRank + centrality) section for the selected file.
fn draw_rank_section(ui: &mut Ui, state: &AppState, path: &str) {
    egui::CollapsingHeader::new(
        egui::RichText::new("Code Rank").monospace().size(9.0).strong(),
    )
    .id_salt("rank_section")
    .default_open(true)
    .show(ui, |ui| {
        let Some(gm) = &state.graph_metrics_report else {
            ui.label(
                egui::RichText::new("No graph metrics data")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let basename = path.rsplit('/').next().unwrap_or(path);
        let Some(&idx) = gm.by_filename.get(basename) else {
            ui.label(
                egui::RichText::new("No centrality data for this file")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let node = &gm.data.nodes[idx];
        // Note: multiple files can share the same basename (e.g. mod.rs)
        if path.ends_with("mod.rs") || path.ends_with("lib.rs") || path.ends_with("main.rs") {
            ui.label(
                egui::RichText::new("(matched by filename)")
                    .monospace()
                    .size(8.0)
                    .color(ui.visuals().weak_text_color()),
            );
        }
        label_row(ui, "PageRank:", &format!("{:.4}", node.pagerank));
        label_row(ui, "Degree Centrality:", &format!("{:.3}", node.degree_centrality));
        label_row(ui, "Betweenness:", &format!("{:.3}", node.betweenness_centrality));
        label_row(ui, "In-degree:", &format!("{}", node.in_degree));
        label_row(ui, "Out-degree:", &format!("{}", node.out_degree));
    });
}

/// Draw the Test Coverage section for the selected file.
fn draw_coverage_section(ui: &mut Ui, state: &AppState, path: &str) {
    egui::CollapsingHeader::new(
        egui::RichText::new("Test Coverage").monospace().size(9.0).strong(),
    )
    .id_salt("coverage_section")
    .default_open(true)
    .show(ui, |ui| {
        let Some(cov) = &state.coverage_report else {
            ui.label(
                egui::RichText::new("Coverage not collected — click Run Coverage in toolbar")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let Some(&idx) = cov.by_path.get(path) else {
            ui.label(
                egui::RichText::new("No coverage data for this file")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let entry = &cov.files[idx];
        label_row(ui, "Lines:", &format!("{:.1}%", entry.summary.lines.percent));
        label_row(ui, "Functions:", &format!("{:.1}%", entry.summary.functions.percent));
    });
}

/// Draw the Clippy Analysis section for the selected file.
fn draw_clippy_section(ui: &mut Ui, state: &AppState, path: &str) {
    egui::CollapsingHeader::new(
        egui::RichText::new("Clippy Analysis").monospace().size(9.0).strong(),
    )
    .id_salt("clippy_section")
    .default_open(true)
    .show(ui, |ui| {
        let Some(clippy) = &state.clippy_report else {
            ui.label(
                egui::RichText::new("No clippy data")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let Some(file_data) = clippy.by_file.get(path) else {
            ui.label(
                egui::RichText::new("No clippy warnings")
                    .monospace()
                    .size(9.0)
                    .color(crate::renderer::colors::STATUS_PASS),
            );
            return;
        };
        ui.label(
            egui::RichText::new(format!("{} warnings", file_data.total))
                .monospace()
                .size(9.0),
        );
        // Show categories sorted by count descending
        let mut cats: Vec<(&String, &u32)> = file_data.by_category.iter().collect();
        cats.sort_by(|a, b| b.1.cmp(a.1));
        for (cat, count) in cats {
            ui.label(
                egui::RichText::new(format!("  {} {}", count, cat))
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
        }
    });
}

/// Draw git diff changes section for the selected file.
/// Shows lines added, lines removed, and commit count from the git diff report.
fn draw_git_diff_section(ui: &mut Ui, state: &AppState, path: &str) {
    egui::CollapsingHeader::new(
        egui::RichText::new("Changes").monospace().size(9.0).strong(),
    )
    .id_salt("git_diff_section")
    .default_open(true)
    .show(ui, |ui| {
        let Some(report) = &state.git_diff_report else {
            ui.label(
                egui::RichText::new("No git diff data — select a diff window in toolbar")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let Some(diff_data) = report.by_file.get(path) else {
            ui.label(
                egui::RichText::new("No changes in this window")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        if diff_data.is_new_file {
            ui.label(
                egui::RichText::new("New file")
                    .monospace()
                    .size(9.0)
                    .color(crate::renderer::colors::STATUS_PASS),
            );
        }
        label_row(ui, "Lines added:", &format!("+{}", diff_data.lines_added));
        label_row(ui, "Lines removed:", &format!("-{}", diff_data.lines_removed));
        label_row(ui, "Commits:", &format!("{}", diff_data.commit_count));

        // Metric deltas (TDG/coverage/clippy changes) will be shown here
        // once analysis snapshot save/load is wired up.
    });
}

/// Draw GSD phase association section for the selected file.
/// Shows which phase the file belongs to (number, name, status). If no association,
/// shows "Not in any GSD phase". Only rendered when ColorMode::GsdPhase is active.
fn draw_gsd_phase_section(ui: &mut Ui, state: &AppState, path: &str) {
    use crate::core::pmat_types::PhaseStatus;
    egui::CollapsingHeader::new(
        egui::RichText::new("GSD Phase").monospace().size(9.0).strong(),
    )
    .id_salt("gsd_phase_section")
    .default_open(true)
    .show(ui, |ui| {
        let Some(report) = &state.gsd_phase_report else {
            ui.label(
                egui::RichText::new("GSD phase data not loaded")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let Some(&phase_idx) = report.by_file.get(path) else {
            ui.label(
                egui::RichText::new("Not in any GSD phase")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let Some(phase) = report.phases.get(phase_idx) else {
            ui.label(
                egui::RichText::new("Phase data missing")
                    .monospace()
                    .size(9.0)
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };
        let status_color = match phase.status {
            PhaseStatus::Completed => crate::renderer::colors::STATUS_PASS,
            PhaseStatus::InProgress => egui::Color32::from_rgb(220, 165, 32),
            PhaseStatus::Planned => egui::Color32::from_rgb(70, 130, 180),
        };
        let status_label = match phase.status {
            PhaseStatus::Completed => "Completed",
            PhaseStatus::InProgress => "In Progress",
            PhaseStatus::Planned => "Planned",
        };
        label_row(ui, "Phase:", &format!("{:02}", phase.number));
        label_row(ui, "Name:", &phase.name);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Status:").monospace().size(9.0));
            ui.colored_label(
                status_color,
                egui::RichText::new(status_label).monospace().size(9.0),
            );
        });
    });
}

/// Render a label+value row.
fn label_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).monospace().size(9.0));
        ui.label(egui::RichText::new(value).monospace().size(9.0));
    });
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
        .id_salt("pmat_categories")
        .show(ui, |ui| {
            for (name, cat) in &repo.categories {
                let status_color = match cat.status.as_str() {
                    "Pass" => crate::renderer::colors::STATUS_PASS,
                    "Warning" => crate::renderer::colors::STATUS_WARN,
                    _ => crate::renderer::colors::STATUS_FAIL,
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
    .id_salt("pmat_components")
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
        .id_salt("pmat_penalties")
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
            crate::renderer::colors::STATUS_FAIL,
            egui::RichText::new(format!("{} critical defect(s)", score.critical_defects_count))
                .monospace()
                .size(9.0),
        );
    }
}

/// Render a single component score row: "Label: value".
fn score_row(ui: &mut Ui, label: &'static str, value: f64) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).monospace().size(9.0));
        ui.label(egui::RichText::new(format!("{value:.1}")).monospace().size(9.0));
    });
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

    /// Panel render contract: report with no matching path for selected file returns gracefully.
    #[test]
    fn file_detail_no_match_for_selected_path() {
        let report = make_test_report(&[("./src/lib.rs", "C", 55.0)]);
        // selected path exists in snapshot but not in PMAT report — graceful None
        let result = report.by_path.get("src/main.rs").and_then(|&idx| report.tdg.files.get(idx));
        assert!(result.is_none(), "Should be None when selected file not in PMAT report");
    }
}
