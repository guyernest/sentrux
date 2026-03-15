//! Panel orchestration — draws all UI panels (toolbar, side panels, status bar).
//!
//! Extracted from update_loop.rs to keep the main loop focused on lifecycle
//! management and reduce fan-out (each panel import is an edge).

use crate::layout::types::ColorMode;
use super::SentruxApp;

/// Outcome of drawing all panels — tells the update loop what actions to take.
pub(crate) struct PanelResult {
    pub layout_changed: bool,
    pub visual_changed: bool,
    pub breadcrumb_changed: bool,
    pub layout_mode_changed: bool,
}

/// Draw the top toolbar and update result flags.
fn draw_toolbar_panel(app: &mut SentruxApp, ctx: &egui::Context, result: &mut PanelResult) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        let old_layout_mode = app.state.layout_mode;
        let old_color_mode = app.state.color_mode;
        let (lc, vc) = crate::app::toolbar::draw_toolbar(ui, &mut app.state);
        if lc {
            result.layout_changed = true;
            if app.state.layout_mode != old_layout_mode {
                result.layout_mode_changed = true;
            }
        }
        if vc {
            result.visual_changed = true;
        }
        // Clear community highlight when color mode changes away from Risk
        if app.state.color_mode != old_color_mode
            && app.state.color_mode != crate::layout::types::ColorMode::Risk
        {
            app.state.community_highlight = None;
        }
        // Color legend strip — rendered on a second row inside the toolbar panel
        draw_color_legend(ui, &app.state);
        // GSD phase navigator bar — rendered below the legend when in GsdPhase mode
        draw_gsd_phase_navigator(ui, &mut app.state);
    });

    // Handle coverage_requested flag — spawn background thread with channel access
    if app.state.coverage_requested {
        app.state.coverage_requested = false;
        maybe_spawn_coverage_thread(app);
    }

    // Handle git_diff_requested flag — spawn background thread with channel access
    if app.state.git_diff_requested {
        app.state.git_diff_requested = false;
        maybe_spawn_git_diff_thread(app);
    }

    // Handle gsd_phase_requested flag — spawn background thread with channel access
    if app.state.gsd_phase_requested {
        app.state.gsd_phase_requested = false;
        maybe_spawn_gsd_phase_thread(app);
    }
}

/// Spawn a background coverage thread if conditions are met.
/// Sets coverage_running=true and sends CoverageReady/CoverageError via scan_msg_tx.
fn maybe_spawn_coverage_thread(app: &mut SentruxApp) {
    let root = match app.state.root_path.clone() {
        Some(r) => r,
        None => return,
    };
    if app.state.coverage_running || app.state.scanning {
        return;
    }
    app.state.coverage_running = true;
    let msg_tx = app.scan_msg_tx.clone();
    match std::thread::Builder::new()
        .name("coverage".into())
        .spawn(move || {
            let result = crate::analysis::pmat_adapter::run_coverage(&root, 0);
            let msg = match result {
                Some(report) => crate::app::channels::ScanMsg::CoverageReady(report),
                None => crate::app::channels::ScanMsg::CoverageError(
                    "cargo-llvm-cov failed or not installed".into()
                ),
            };
            let _ = msg_tx.send(msg);
        })
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[app] failed to spawn coverage thread: {}", e);
            app.state.coverage_running = false;
        }
    }
}

/// Spawn a background git diff thread if conditions are met.
/// Sets git_diff_running=true and sends GitDiffReady/GitDiffError via scan_msg_tx.
fn maybe_spawn_git_diff_thread(app: &mut SentruxApp) {
    let root = match app.state.root_path.clone() {
        Some(r) => r,
        None => return,
    };
    if app.state.git_diff_running || app.state.scanning {
        return;
    }
    app.state.git_diff_running = true;
    let msg_tx = app.scan_msg_tx.clone();
    let window = app.state.git_diff_window.clone();
    match std::thread::Builder::new()
        .name("git-diff".into())
        .spawn(move || {
            match crate::analysis::git_diff_adapter::compute_git_diff_report(&root, window) {
                Ok(report) => { let _ = msg_tx.send(crate::app::channels::ScanMsg::GitDiffReady(report)); }
                Err(e) => { let _ = msg_tx.send(crate::app::channels::ScanMsg::GitDiffError(e)); }
            }
        })
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[app] failed to spawn git-diff thread: {}", e);
            app.state.git_diff_running = false;
        }
    }
}

/// Spawn a background GSD phase thread if conditions are met.
/// Sets gsd_phase_running=true and sends GsdPhaseReady/GsdPhaseError via scan_msg_tx.
fn maybe_spawn_gsd_phase_thread(app: &mut SentruxApp) {
    let root = match app.state.root_path.clone() {
        Some(r) => r,
        None => return,
    };
    if app.state.gsd_phase_running || app.state.scanning {
        return;
    }
    app.state.gsd_phase_running = true;
    let msg_tx = app.scan_msg_tx.clone();
    match std::thread::Builder::new()
        .name("gsd-phase".into())
        .spawn(move || {
            let msg = match crate::analysis::gsd_phase_adapter::parse_gsd_phases(&root) {
                Some(report) => crate::app::channels::ScanMsg::GsdPhaseReady(report),
                None => crate::app::channels::ScanMsg::GsdPhaseError(
                    "Failed to parse .planning/ directory".into()
                ),
            };
            let _ = msg_tx.send(msg);
        })
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[app] failed to spawn gsd-phase thread: {}", e);
            app.state.gsd_phase_running = false;
        }
    }
}

/// Draw settings panel if open, updating result flags.
fn draw_settings_if_open(app: &mut SentruxApp, ctx: &egui::Context, result: &mut PanelResult) {
    if !app.state.settings_open { return; }
    let (s_layout, s_visual) = crate::app::settings_panel::draw_settings_panel(
        ctx,
        &mut app.state.settings,
        &mut app.state.settings_open,
    );
    if s_layout { result.layout_changed = true; }
    if s_visual { result.visual_changed = true; }
}

/// Draw optional side panels (settings, metrics, activity, DSM).
fn draw_side_panels(app: &mut SentruxApp, ctx: &egui::Context, result: &mut PanelResult) {
    draw_settings_if_open(app, ctx, result);

    if app.state.snapshot.is_some() {
        crate::app::panels::metrics_panel::draw_metrics_panel(ctx, &mut app.state);
    }

    if app.state.activity_panel_open && app.state.snapshot.is_some()
        && crate::app::panels::activity_panel::draw_activity_panel(ctx, &mut app.state) {
            result.visual_changed = true;
        }

}

/// Draw all non-canvas panels. Returns what changed so update_loop can act.
pub(crate) fn draw_all_panels(app: &mut SentruxApp, ctx: &egui::Context) -> PanelResult {
    let mut result = PanelResult {
        layout_changed: false,
        visual_changed: false,
        breadcrumb_changed: false,
        layout_mode_changed: false,
    };

    draw_toolbar_panel(app, ctx, &mut result);
    draw_side_panels(app, ctx, &mut result);

    egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
        crate::app::status_bar::draw_status_bar(ui, &app.state);
    });

    if !app.state.drill_stack.is_empty() {
        egui::TopBottomPanel::top("breadcrumb").show(ctx, |ui| {
            result.breadcrumb_changed = crate::app::breadcrumb::draw_breadcrumb(ui, &mut app.state);
        });
    }

    result
}

/// Draw the progress overlay on the canvas when scanning.
pub(crate) fn draw_progress(ui: &mut egui::Ui, state: &crate::app::state::AppState, has_render: bool) {
    crate::app::progress::draw_progress_overlay(ui, state, has_render);
}

/// Draw a small colored swatch rectangle inline at the current cursor position.
fn draw_swatch(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color);
}

/// Draw a horizontal gradient strip from `color_a` to `color_b`.
fn draw_gradient_strip(ui: &mut egui::Ui, color_a: egui::Color32, color_b: egui::Color32) {
    let steps = 12;
    let step_w = 80.0 / steps as f32;
    let (strip_rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 10.0), egui::Sense::hover());
    let painter = ui.painter();
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let r = (color_a.r() as f32 + t * (color_b.r() as f32 - color_a.r() as f32)) as u8;
        let g = (color_a.g() as f32 + t * (color_b.g() as f32 - color_a.g() as f32)) as u8;
        let b = (color_a.b() as f32 + t * (color_b.b() as f32 - color_a.b() as f32)) as u8;
        let color = egui::Color32::from_rgb(r, g, b);
        let x = strip_rect.left() + i as f32 * step_w;
        let cell = egui::Rect::from_min_size(
            egui::pos2(x, strip_rect.top()),
            egui::vec2(step_w + 0.5, strip_rect.height()),
        );
        painter.rect_filled(cell, 0.0, color);
    }
}

/// Color legend for GitDiff mode.
fn draw_git_diff_legend(ui: &mut egui::Ui, has_report: bool) {
    use crate::renderer::colors::{git_diff_intensity_color, git_diff_new_file_color};
    draw_swatch(ui, crate::renderer::colors::NO_DATA_GRAY);
    ui.add_space(2.0);
    ui.label(egui::RichText::new("unchanged").small().weak());
    ui.add_space(8.0);
    draw_gradient_strip(ui, git_diff_intensity_color(0.0), git_diff_intensity_color(1.0));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("few \u{2192} many changes").small().weak());
    ui.add_space(8.0);
    draw_swatch(ui, git_diff_new_file_color());
    ui.add_space(2.0);
    ui.label(egui::RichText::new("new file").small().weak());
    if !has_report {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("(no data \u{2014} select a window)").small().weak()
            .color(egui::Color32::from_rgb(150, 140, 100)));
    }
}

/// Color legend for TdgGrade mode — grade badges A+ through F.
fn draw_tdg_legend(ui: &mut egui::Ui) {
    use crate::renderer::colors::tdg_grade_color;
    let grades = [("A+", "APLus"), ("A", "A"), ("B", "B"), ("C", "C"), ("D", "D"), ("F", "F")];
    for (display, grade_key) in grades {
        draw_swatch(ui, tdg_grade_color(grade_key));
        ui.add_space(2.0);
        ui.label(egui::RichText::new(display).small().weak());
        ui.add_space(6.0);
    }
}

/// Color legend for Coverage mode.
fn draw_coverage_legend(ui: &mut egui::Ui) {
    use crate::renderer::colors::coverage_color;
    draw_gradient_strip(ui, coverage_color(100.0), coverage_color(0.0));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("well-covered \u{2192} uncovered").small().weak());
    ui.add_space(8.0);
    draw_swatch(ui, crate::renderer::colors::NO_DATA_GRAY);
    ui.add_space(2.0);
    ui.label(egui::RichText::new("no data").small().weak());
}

/// Color legend for Risk mode.
fn draw_risk_legend(ui: &mut egui::Ui) {
    draw_gradient_strip(
        ui,
        egui::Color32::from_rgb(30, 180, 40),
        egui::Color32::from_rgb(244, 67, 54),
    );
    ui.add_space(2.0);
    ui.label(egui::RichText::new("safe \u{2192} risky").small().weak());
}

/// Color legend for GsdPhase mode.
fn draw_gsd_phase_legend(ui: &mut egui::Ui, state: &crate::app::state::AppState) {
    use crate::renderer::colors::gsd_phase_color;
    use crate::core::pmat_types::PhaseStatus;
    draw_swatch(ui, gsd_phase_color(PhaseStatus::Completed));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Completed").small().weak());
    ui.add_space(8.0);
    draw_swatch(ui, gsd_phase_color(PhaseStatus::InProgress));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("In Progress").small().weak());
    ui.add_space(8.0);
    draw_swatch(ui, gsd_phase_color(PhaseStatus::Planned));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Planned").small().weak());
    ui.add_space(8.0);
    draw_swatch(ui, crate::renderer::colors::NO_DATA_GRAY);
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Not in any phase").small().weak());

    // Coverage stat
    if let Some(report) = &state.gsd_phase_report {
        let total_files = state.file_index.len();
        if total_files > 0 {
            let covered = report.by_file.len();
            let pct = (covered as f32 / total_files as f32 * 100.0) as u32;
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(format!("{}% phase coverage", pct))
                    .small()
                    .weak()
                    .color(egui::Color32::from_rgb(150, 155, 165)),
            );
        }
    }
}

/// Draw the GSD phase navigator proportional bar.
///
/// Renders a horizontal bar where each segment width is proportional to the
/// number of files in that phase. Clicking a segment sets GitDiff to that
/// phase's commit range. Hovering shows a tooltip with phase details.
pub(crate) fn draw_gsd_phase_navigator(ui: &mut egui::Ui, state: &mut crate::app::state::AppState) {
    use crate::renderer::colors::gsd_phase_color;
    use crate::core::pmat_types::PhaseStatus;
    use crate::metrics::evo::git_walker::DiffWindow;

    // Guard: only render in GsdPhase mode
    if state.color_mode != ColorMode::GsdPhase {
        return;
    }

    if state.gsd_phase_running {
        ui.label(egui::RichText::new("Scanning GSD phases...").small().weak());
        return;
    }

    let report = match &state.gsd_phase_report {
        Some(r) => r.clone(),
        None => {
            ui.label(egui::RichText::new("No .planning/ directory found").small().weak());
            return;
        }
    };

    if report.phases.is_empty() {
        ui.label(egui::RichText::new("No phases found in .planning/").small().weak());
        return;
    }

    let total_files: usize = report.phases.iter().map(|p| p.files.len()).sum();
    let total_width = ui.available_width();
    let bar_height = 18.0;
    const MIN_SEG_WIDTH: f32 = 40.0;

    // Allocate space for the whole bar
    let (bar_rect, _) = ui.allocate_exact_size(
        egui::vec2(total_width, bar_height),
        egui::Sense::hover(),
    );

    let painter = ui.painter();

    // Compute segment widths
    let n = report.phases.len();
    let mut widths: Vec<f32> = report.phases.iter().map(|phase| {
        if total_files == 0 {
            total_width / n as f32
        } else {
            (phase.files.len() as f32 / total_files as f32) * total_width
        }
    }).collect();

    // Apply minimum width, then redistribute remaining space
    let forced_min: f32 = widths.iter().map(|&w| if w < MIN_SEG_WIDTH { MIN_SEG_WIDTH } else { w }).sum();
    if forced_min > total_width {
        // Not enough space: just give each segment equal width
        let eq = total_width / n as f32;
        for w in &mut widths {
            *w = eq;
        }
    } else {
        // Apply minimums and scale remaining proportionally
        let mut remaining = total_width;
        let mut free_indices: Vec<usize> = Vec::new();
        let mut free_natural: f32 = 0.0;
        for (i, w) in widths.iter_mut().enumerate() {
            if *w < MIN_SEG_WIDTH {
                *w = MIN_SEG_WIDTH;
                remaining -= MIN_SEG_WIDTH;
            } else {
                free_indices.push(i);
                free_natural += *w;
            }
        }
        // Scale free segments to fill remaining space
        if free_natural > 0.0 {
            for &i in &free_indices {
                widths[i] = widths[i] / free_natural * remaining;
            }
        }
    }

    // Draw segments and handle interaction
    let mut x = bar_rect.left();
    let mut new_selected: Option<usize> = state.selected_phase_idx;
    let mut new_git_diff_window: Option<DiffWindow> = None;
    let mut new_git_diff_requested = false;

    for (idx, phase) in report.phases.iter().enumerate() {
        let w = widths[idx];
        let seg_rect = egui::Rect::from_min_size(
            egui::pos2(x, bar_rect.top()),
            egui::vec2(w, bar_height),
        );

        // Fill color
        let fill_color = gsd_phase_color(phase.status);
        painter.rect_filled(seg_rect, 0.0, fill_color);

        // Border for in-progress (bright) or selected (contrasting)
        let is_current = phase.status == PhaseStatus::InProgress;
        let is_selected = state.selected_phase_idx == Some(idx);
        if is_current {
            painter.rect_stroke(
                seg_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 80)),
                egui::StrokeKind::Inside,
            );
        } else if is_selected {
            painter.rect_stroke(
                seg_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)),
                egui::StrokeKind::Inside,
            );
        }

        // Label inside segment
        let font_id = egui::FontId::monospace(9.0);
        let label_color = egui::Color32::WHITE;
        let short_label = format!("P{}", phase.number);
        let text_pos = egui::pos2(x + 4.0, bar_rect.center().y - 3.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            &short_label,
            font_id.clone(),
            label_color,
        );
        if w > 100.0 {
            // Show phase name below the number
            let name_preview: String = phase.name.chars().take(12).collect();
            let name_text = if phase.name.len() > 12 { format!("{}..", name_preview) } else { name_preview };
            painter.text(
                egui::pos2(x + 4.0, bar_rect.center().y + 5.0),
                egui::Align2::LEFT_CENTER,
                name_text,
                egui::FontId::monospace(8.0),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
            );
        }

        // Interaction: use ui.interact() on the segment rect
        let seg_id = ui.id().with(("phase_seg", idx));
        let seg_response = ui.interact(seg_rect, seg_id, egui::Sense::click());

        if seg_response.clicked() {
            new_selected = Some(idx);
            if let Some((ref from, ref to)) = phase.commit_range {
                new_git_diff_window = Some(DiffWindow::CommitRange {
                    from: from.clone(),
                    to: to.clone(),
                });
                new_git_diff_requested = true;
            }
        }

        // Hover tooltip
        let status_label = match phase.status {
            PhaseStatus::Completed => "Completed",
            PhaseStatus::InProgress => "In Progress",
            PhaseStatus::Planned => "Planned",
        };
        let tooltip = format!(
            "Phase {}: {}\nGoal: {}\nStatus: {}\nFiles: {}",
            phase.number,
            phase.name,
            phase.goal,
            status_label,
            phase.files.len(),
        );
        seg_response.on_hover_text(egui::RichText::new(tooltip).monospace().size(10.0));

        x += w;
    }

    // Apply mutations after iteration (borrow checker: report was cloned above)
    state.selected_phase_idx = new_selected;
    if let Some(window) = new_git_diff_window {
        state.git_diff_window = window;
        state.git_diff_requested = new_git_diff_requested;
    }
}

/// Draw a per-mode color legend below the toolbar.
///
/// Only rendered for modes that have a meaningful color scale (GitDiff, TdgGrade,
/// Coverage, Risk). Other modes (Language, Heat, Git, Monochrome) return early.
pub(crate) fn draw_color_legend(ui: &mut egui::Ui, state: &crate::app::state::AppState) {
    match state.color_mode {
        ColorMode::GitDiff => {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                draw_git_diff_legend(ui, state.git_diff_report.is_some());
            });
        }
        ColorMode::TdgGrade => {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                draw_tdg_legend(ui);
            });
        }
        ColorMode::Coverage => {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                draw_coverage_legend(ui);
            });
        }
        ColorMode::Risk => {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                draw_risk_legend(ui);
            });
        }
        ColorMode::GsdPhase => {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                draw_gsd_phase_legend(ui, state);
            });
        }
        _ => {}
    }
}
