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
    let window = app.state.git_diff_window;
    match std::thread::Builder::new()
        .name("git-diff".into())
        .spawn(move || {
            crate::analysis::git_diff_adapter::spawn_git_diff_thread(root, window, msg_tx);
        })
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[app] failed to spawn git-diff thread: {}", e);
            app.state.git_diff_running = false;
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
    draw_swatch(ui, egui::Color32::from_rgb(50, 52, 55));
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
    draw_swatch(ui, egui::Color32::from_rgb(50, 52, 55));
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
        _ => {}
    }
}
