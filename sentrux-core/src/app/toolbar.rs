//! Toolbar UI — mode selectors, filter controls, and scan progress display.
//!
//! Returns `(layout_changed, visual_changed)` so the caller knows whether
//! to trigger a re-layout or just a repaint.

use crate::layout::types::{ColorMode, EdgeFilter, FocusMode, LayoutMode, ScaleMode, SizeMode};
use crate::core::settings::Theme;
use crate::metrics::evo::git_walker::DiffWindow;
use super::state::AppState;

/// Draw the toolbar panel. Returns (layout_changed, visual_changed).
/// layout_changed = size/scale/layout mode changed (needs re-layout).
/// visual_changed = color/theme/edge/focus changed (needs repaint only).
pub fn draw_toolbar(ui: &mut egui::Ui, state: &mut AppState) -> (bool, bool) {
    let mut layout_changed = false;
    let mut visual_changed = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        draw_open_folder(ui, state);

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(2.0);

        draw_structure_group(ui, state, &mut layout_changed);

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        draw_visual_group(ui, state, &mut visual_changed);

        draw_git_diff_controls(ui, state);
        draw_gsd_phase_controls(ui, state);

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        draw_filter_group(ui, state, &mut layout_changed, &mut visual_changed);

        draw_scan_progress(ui, state);
    });

    (layout_changed, visual_changed)
}

/// Open folder button — sets a flag for app.rs to handle on a background
/// thread, avoiding blocking the UI event loop.
fn draw_open_folder(ui: &mut egui::Ui, state: &mut AppState) {
    if ui.button("Open Folder").clicked() {
        state.folder_picker_requested = true;
    }
}

/// Structure group: Layout mode, Size mode, Scale mode combo boxes.
fn draw_structure_group(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    draw_layout_mode_combo(ui, state, layout_changed);
    draw_size_mode_combo(ui, state, layout_changed);
    draw_scale_mode_combo(ui, state, layout_changed);
}

/// Layout mode combo box (Treemap / Blueprint).
fn draw_layout_mode_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    ui.label(egui::RichText::new("Layout").small().weak());
    let layout_label = match state.layout_mode {
        LayoutMode::Treemap => "Treemap",
        LayoutMode::Blueprint => "Blueprint",
    };
    egui::ComboBox::from_id_salt("layout_mode")
        .selected_text(layout_label)
        .width(80.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(&mut state.layout_mode, LayoutMode::Treemap, "Treemap")
                .changed()
            {
                *layout_changed = true;
            }
            if ui
                .selectable_value(&mut state.layout_mode, LayoutMode::Blueprint, "Blueprint")
                .changed()
            {
                *layout_changed = true;
            }
        });
}

/// Map SizeMode to its display label.
fn size_mode_label(mode: SizeMode) -> &'static str {
    match mode {
        SizeMode::Lines => "Lines",
        SizeMode::Logic => "Logic",
        SizeMode::Funcs => "Funcs",
        SizeMode::Comments => "Comments",
        SizeMode::Blanks => "Blanks",
        SizeMode::Heat => "Heat",
        SizeMode::PageRank => "PageRank",
        SizeMode::Centrality => "Centrality",
        SizeMode::ClippyCount => "Clippy",
        SizeMode::Uniform => "Uniform",
    }
}

/// All SizeMode variants in display order.
const SIZE_MODES: &[SizeMode] = &[
    SizeMode::Lines, SizeMode::Logic, SizeMode::Funcs,
    SizeMode::Comments, SizeMode::Blanks, SizeMode::Heat,
    SizeMode::PageRank, SizeMode::Centrality, SizeMode::ClippyCount,
    SizeMode::Uniform,
];

/// Size mode combo box (Lines / Logic / Funcs / ...).
fn draw_size_mode_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    ui.label(egui::RichText::new("size:").small().weak());
    egui::ComboBox::from_id_salt("size_mode")
        .selected_text(size_mode_label(state.size_mode))
        .width(70.0)
        .show_ui(ui, |ui| {
            for &mode in SIZE_MODES {
                if ui.selectable_value(&mut state.size_mode, mode, size_mode_label(mode)).changed() {
                    *layout_changed = true;
                }
            }
        });
}

/// Scale mode combo box (Linear / Sqrt / Log / Smooth).
fn draw_scale_mode_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    let scale_label = match state.scale_mode {
        ScaleMode::Linear => "Lin",
        ScaleMode::Sqrt => "Sqrt",
        ScaleMode::Log => "Log",
        ScaleMode::Smooth => "Smo",
    };
    egui::ComboBox::from_id_salt("scale_mode")
        .selected_text(scale_label)
        .width(50.0)
        .show_ui(ui, |ui| {
            for mode in [ScaleMode::Linear, ScaleMode::Sqrt, ScaleMode::Log, ScaleMode::Smooth] {
                let label = match mode {
                    ScaleMode::Linear => "Linear",
                    ScaleMode::Sqrt => "Sqrt",
                    ScaleMode::Log => "Log",
                    ScaleMode::Smooth => "Smooth",
                };
                if ui.selectable_value(&mut state.scale_mode, mode, label).changed() {
                    *layout_changed = true;
                }
            }
        });
}

/// Check whether cargo-llvm-cov is available (cached in pmat_adapter).
fn llvm_cov_available() -> bool {
    crate::analysis::pmat_adapter::check_llvm_cov_available()
}

/// Visual group: Color mode (with Coverage gating) and Theme combo boxes.
fn draw_visual_group(ui: &mut egui::Ui, state: &mut AppState, visual_changed: &mut bool) {
    ui.label(egui::RichText::new("color:").small().weak());
    let color_label = state.color_mode.label();
    let available_modes = ColorMode::ALL;
    let prev_color_mode = state.color_mode;
    egui::ComboBox::from_id_salt("color_mode")
        .selected_text(color_label)
        .width(80.0)
        .show_ui(ui, |ui| {
            for &mode in available_modes {
                if mode == ColorMode::Coverage && state.coverage_report.is_none() {
                    // Grayed out: coverage data not yet collected
                    let tooltip = if llvm_cov_available() {
                        "Run coverage first (button below)"
                    } else {
                        "cargo-llvm-cov not installed — run: cargo install cargo-llvm-cov"
                    };
                    ui.add_enabled(false, egui::Button::new(
                        egui::RichText::new(mode.label()).weak()
                    )).on_disabled_hover_text(tooltip);
                } else if ui.selectable_value(&mut state.color_mode, mode, mode.label()).changed() {
                    *visual_changed = true;
                }
            }
        });
    // Auto-trigger git diff computation when switching TO GitDiff mode
    if state.color_mode == ColorMode::GitDiff
        && prev_color_mode != ColorMode::GitDiff
        && state.git_diff_report.is_none()
        && !state.git_diff_running
        && !state.scanning
    {
        state.git_diff_requested = true;
    }
    // Auto-trigger GSD phase parse when switching TO GsdPhase mode
    if state.color_mode == ColorMode::GsdPhase
        && prev_color_mode != ColorMode::GsdPhase
        && state.gsd_phase_report.is_none()
        && !state.gsd_phase_running
        && !state.scanning
    {
        state.gsd_phase_requested = true;
    }

    let theme_label = state.theme.label();
    egui::ComboBox::from_id_salt("theme")
        .selected_text(theme_label)
        .width(70.0)
        .show_ui(ui, |ui| {
            for &theme in Theme::ALL {
                if ui
                    .selectable_value(&mut state.theme, theme, theme.label())
                    .changed()
                {
                    state.set_theme(theme);
                    *visual_changed = true;
                }
            }
        });

    draw_coverage_button(ui, state);
}

/// Draw the "Run Coverage" button. Sets coverage_requested flag when clicked.
/// Disabled while coverage is running or no folder is open.
fn draw_coverage_button(ui: &mut egui::Ui, state: &mut AppState) {
    let can_run = state.root_path.is_some() && !state.coverage_running && !state.scanning;
    let btn_label = if state.coverage_running { "Running..." } else { "Run Coverage" };
    let btn = ui.add_enabled(
        can_run,
        egui::Button::new(egui::RichText::new(btn_label).small()),
    );
    if btn.clicked() {
        state.coverage_requested = true;
    }
    if !llvm_cov_available() {
        btn.on_disabled_hover_text("cargo-llvm-cov not installed — run: cargo install cargo-llvm-cov");
    }
}

/// Git diff window selector row — only shown when ColorMode::GitDiff is active.
///
/// Renders preset buttons (15m/1h/1d/1w/tag/1c/5c) plus a custom commit count
/// input. Clicking a preset or submitting the custom N sets git_diff_requested.
pub fn draw_git_diff_controls(ui: &mut egui::Ui, state: &mut AppState) {
    if state.color_mode != ColorMode::GitDiff {
        return;
    }
    ui.separator();
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Window:").small().weak());

    for (window, label) in DiffWindow::preset_slice() {
        let selected = state.git_diff_window == *window;
        if ui.selectable_label(selected, *label).clicked() {
            state.git_diff_window = window.clone();
            state.git_diff_requested = true;
        }
    }

    ui.add_space(4.0);
    ui.label(egui::RichText::new("N:").small().weak());
    let drag = egui::DragValue::new(&mut state.git_diff_custom_n)
        .range(1..=999u32)
        .speed(1.0);
    let response = ui.add(drag);
    let go_clicked = ui.button(egui::RichText::new("go").small()).clicked();
    if go_clicked || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
        state.git_diff_window = DiffWindow::CommitCount(state.git_diff_custom_n);
        state.git_diff_requested = true;
    }

    if state.git_diff_running {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("computing...")
                .small()
                .weak()
                .color(egui::Color32::from_rgb(120, 160, 200)),
        );
    }
}

/// GSD phase controls row — only shown when ColorMode::GsdPhase is active.
///
/// Shows a "Refresh" button to re-trigger GSD phase parsing, plus a spinner
/// when parsing is in progress.
pub fn draw_gsd_phase_controls(ui: &mut egui::Ui, state: &mut AppState) {
    if state.color_mode != ColorMode::GsdPhase {
        return;
    }
    ui.separator();
    ui.add_space(2.0);
    ui.label(egui::RichText::new("GSD:").small().weak());

    let can_refresh = state.root_path.is_some() && !state.gsd_phase_running && !state.scanning;
    let btn_label = if state.gsd_phase_running { "Scanning..." } else { "Refresh" };
    let btn = ui.add_enabled(
        can_refresh,
        egui::Button::new(egui::RichText::new(btn_label).small()),
    );
    if btn.clicked() {
        state.gsd_phase_requested = true;
    }

    if state.gsd_phase_running {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("scanning...")
                .small()
                .weak()
                .color(egui::Color32::from_rgb(120, 160, 200)),
        );
    }
}

/// Filter group: Focus mode, Edge filter, edge/DSM/activity toggles.
fn draw_filter_group(
    ui: &mut egui::Ui,
    state: &mut AppState,
    layout_changed: &mut bool,
    visual_changed: &mut bool,
) {
    draw_focus_combo(ui, state, layout_changed);
    draw_edge_filter_combo(ui, state, visual_changed);
    draw_toggle_buttons(ui, state);
}

/// Focus mode combo box (All / EntryPoints / Directory / Language).
fn draw_focus_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    ui.label(egui::RichText::new("Focus").small().weak());
    let focus_label = state.focus_mode.label();
    egui::ComboBox::from_id_salt("focus_mode")
        .selected_text(focus_label)
        .width(85.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(matches!(state.focus_mode, FocusMode::All), "All Files")
                .clicked()
            {
                state.focus_mode = FocusMode::All;
                *layout_changed = true;
            }
            if ui
                .selectable_label(
                    matches!(state.focus_mode, FocusMode::EntryPoints),
                    "Entry Points",
                )
                .clicked()
            {
                state.focus_mode = FocusMode::EntryPoints;
                *layout_changed = true;
            }
            draw_focus_dir_items(ui, state, layout_changed);
            draw_focus_lang_items(ui, state, layout_changed);
        });
}

/// Directory items inside the focus combo dropdown.
fn draw_focus_dir_items(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    if state.top_dirs.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("Directories").small().weak());
    let mut clicked_dir = None;
    for i in 0..state.top_dirs.len() {
        let dir = &state.top_dirs[i];
        let is_sel = matches!(&state.focus_mode, FocusMode::Directory(d) if d == dir);
        if ui.selectable_label(is_sel, dir).clicked() {
            clicked_dir = Some(i);
        }
    }
    if let Some(i) = clicked_dir {
        state.focus_mode = FocusMode::Directory(state.top_dirs[i].clone());
        *layout_changed = true;
    }
}

/// Language items inside the focus combo dropdown.
fn draw_focus_lang_items(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    if state.languages.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("Languages").small().weak());
    let mut clicked_lang = None;
    for i in 0..state.languages.len() {
        let lang = &state.languages[i];
        let is_sel = matches!(&state.focus_mode, FocusMode::Language(l) if l == lang);
        if ui.selectable_label(is_sel, lang).clicked() {
            clicked_lang = Some(i);
        }
    }
    if let Some(i) = clicked_lang {
        state.focus_mode = FocusMode::Language(state.languages[i].clone());
        *layout_changed = true;
    }
}

/// Edge filter combo box.
fn draw_edge_filter_combo(ui: &mut egui::Ui, state: &mut AppState, visual_changed: &mut bool) {
    let ctx_label = state.edge_filter.label();
    egui::ComboBox::from_id_salt("edge_filter")
        .selected_text(ctx_label)
        .width(75.0)
        .show_ui(ui, |ui| {
            for &filter in EdgeFilter::ALL {
                if ui.selectable_value(&mut state.edge_filter, filter, filter.label()).changed() {
                    *visual_changed = true;
                }
            }
        });
}

/// Color for an active/inactive toggle state.
fn toggle_color(active: bool, active_color: egui::Color32) -> egui::Color32 {
    if active { active_color } else { egui::Color32::from_rgb(120, 120, 120) }
}

/// Draw the show-all-edges toggle button.
fn draw_edge_toggle(ui: &mut egui::Ui, state: &mut AppState) {
    let edge_icon = if state.show_all_edges { "\u{26A1}" } else { "\u{25C7}" };
    let color = toggle_color(state.show_all_edges, egui::Color32::from_rgb(220, 180, 80));
    let edge_btn = ui.add(
        egui::Button::new(egui::RichText::new(edge_icon).monospace().color(color))
            .fill(egui::Color32::TRANSPARENT),
    );
    if edge_btn.clicked() { state.show_all_edges = !state.show_all_edges; }
    let tip = if state.show_all_edges { "Showing all edges \u{2014} click to show only on hover" }
        else { "Edges shown on hover \u{2014} click to show all" };
    edge_btn.on_hover_text(tip);
}


/// Draw the activity panel toggle button.
fn draw_activity_toggle(ui: &mut egui::Ui, state: &mut AppState) {
    let act_label = if state.activity_panel_open { "\u{258C}" } else { "\u{2590}" };
    let color = toggle_color(state.activity_panel_open, egui::Color32::from_rgb(220, 180, 80));
    let act_btn = ui.add(egui::Button::new(egui::RichText::new(act_label).monospace().color(color)));
    if act_btn.on_hover_text("Activity Panel").clicked() { state.activity_panel_open = !state.activity_panel_open; }
}

/// Toggle buttons for show-all-edges and activity panel.
fn draw_toggle_buttons(ui: &mut egui::Ui, state: &mut AppState) {
    draw_edge_toggle(ui, state);
    draw_activity_toggle(ui, state);
}

/// Scan progress spinner and percentage indicator.
fn draw_scan_progress(ui: &mut egui::Ui, state: &AppState) {
    if state.scanning {
        ui.add_space(4.0);
        // Rotating block chars: ▖▘▝▗
        let frames = ['▖', '▘', '▝', '▗'];
        let idx = ((ui.input(|i| i.time) * 6.0) as usize) % frames.len();
        ui.label(egui::RichText::new(frames[idx].to_string()).monospace());
        ui.label(
            egui::RichText::new(format!("{}  {}%", state.scan_step, state.scan_pct))
                .small()
                .weak(),
        );
    }
}
