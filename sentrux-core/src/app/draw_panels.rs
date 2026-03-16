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
        // Timeline navigator bar — rendered below the legend when GSD phase data exists
        draw_timeline_navigator(ui, &mut app.state);
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

    // Handle snapshot_write_requested flag — spawn background thread with channel access
    if app.state.snapshot_write_requested {
        app.state.snapshot_write_requested = false;
        maybe_spawn_snapshot_writer_thread(app);
    }

    // Handle delta_requested flag — spawn background thread with channel access
    if app.state.delta_requested {
        app.state.delta_requested = false;
        maybe_spawn_delta_thread(app);
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

/// Spawn a background snapshot writer thread if conditions are met.
/// Sets snapshot_write_running=true and sends SnapshotStored via scan_msg_tx.
/// Called when snapshot_write_requested flag is set (after scan completes).
fn maybe_spawn_snapshot_writer_thread(app: &mut SentruxApp) {
    if app.state.snapshot_write_running {
        return;
    }
    let root = match app.state.root_path.clone() {
        Some(r) => r,
        None => return,
    };
    let pmat = app.state.pmat_report.clone();
    let coverage = app.state.coverage_report.clone();
    let clippy = app.state.clippy_report.clone();
    app.state.snapshot_write_running = true;
    let msg_tx = app.scan_msg_tx.clone();
    match std::thread::Builder::new()
        .name("snapshot-writer".into())
        .spawn(move || {
            let msg = match crate::analysis::snapshot_writer::write_analysis_snapshot(
                &root, &pmat, &coverage, &clippy,
            ) {
                Ok(path) => crate::app::channels::ScanMsg::SnapshotStored(path),
                Err(e) => {
                    eprintln!("[snapshot] write error: {}", e);
                    crate::app::channels::ScanMsg::SnapshotStored(String::new())
                }
            };
            let _ = msg_tx.send(msg);
        })
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[app] failed to spawn snapshot-writer thread: {}", e);
            app.state.snapshot_write_running = false;
        }
    }
}

/// Spawn a background delta computation thread if conditions are met.
/// Sets delta_running=true and sends DeltaReady/DeltaError via scan_msg_tx.
/// Called when delta_requested flag is set (after timeline selection changes).
fn maybe_spawn_delta_thread(app: &mut SentruxApp) {
    if app.state.delta_running {
        return;
    }
    let root = match app.state.root_path.clone() {
        Some(r) => r,
        None => return,
    };
    let selection = match app.state.timeline_selection.clone() {
        Some(s) => s,
        None => {
            // No selection — clear any stale delta report
            app.state.timeline_delta_report = None;
            return;
        }
    };
    let pmat = app.state.pmat_report.clone();
    let coverage = app.state.coverage_report.clone();
    let clippy = app.state.clippy_report.clone();
    app.state.delta_running = true;
    let msg_tx = app.scan_msg_tx.clone();
    match std::thread::Builder::new()
        .name("delta-compute".into())
        .spawn(move || {
            let baseline_opt =
                crate::analysis::snapshot_writer::load_nearest_snapshot(&root, selection.epoch_start);
            let report = match baseline_opt {
                Some(baseline) => crate::analysis::snapshot_writer::compute_delta_report(
                    &root, &baseline, &pmat, &coverage, &clippy,
                ),
                None => {
                    // No baseline snapshot — return empty report (no arrows, correct per RESEARCH.md pitfall 3)
                    crate::core::pmat_types::TimelineDeltaReport {
                        by_file: std::collections::HashMap::new(),
                        baseline_epoch: selection.epoch_start,
                    }
                }
            };
            let _ = msg_tx.send(crate::app::channels::ScanMsg::DeltaReady(report));
        })
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[app] failed to spawn delta-compute thread: {}", e);
            app.state.delta_running = false;
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

/// Draw the 3-tier timeline navigator bar.
///
/// Renders stacked rows: time ticks, optional milestone row (only when > 1 milestone),
/// phases, and commits. Each row uses equal-width segments. Click sets timeline_selection.
/// Visible whenever gsd_phase_report is Some, regardless of color mode.
pub(crate) fn draw_timeline_navigator(ui: &mut egui::Ui, state: &mut crate::app::state::AppState) {
    use crate::renderer::colors::gsd_phase_color;
    use crate::core::pmat_types::{TimelineSelection, TimelineSelectionKind};

    // Guard: only render when GSD phase data is available
    if state.gsd_phase_report.is_none() {
        return;
    }

    if state.gsd_phase_running {
        ui.label(egui::RichText::new("Loading timeline...").small().weak());
        return;
    }

    // Take references to avoid per-frame cloning. Mutations collected in locals, applied at end.
    let report = state.gsd_phase_report.as_ref().unwrap();
    let commits = &report.commits;
    let milestones = &state.milestone_infos;
    let current_selection = &state.timeline_selection;

    let total_width = ui.available_width();

    // ── Row 1: Time ticks (12px) ──────────────────────────────────────────

    let mut new_selection: Option<Option<TimelineSelection>> = None; // Some(None) = deselect, Some(Some(x)) = select

    if !commits.is_empty() {
        let epoch_min = commits.first().map(|c| c.epoch).unwrap_or(0);
        let epoch_max = commits.last().map(|c| c.epoch).unwrap_or(0);
        let span = epoch_max - epoch_min;
        let tick_interval = choose_tick_granularity_secs(span);

        let tick_height = 12.0;
        let (tick_rect, _) = ui.allocate_exact_size(
            egui::vec2(total_width, tick_height),
            egui::Sense::hover(),
        );
        let painter = ui.painter();

        if tick_interval > 0 && span > 0 {
            // First tick at the first multiple of tick_interval >= epoch_min
            let mut tick_epoch = epoch_min - (epoch_min % tick_interval);
            if tick_epoch < epoch_min {
                tick_epoch += tick_interval;
            }

            let n_commits = commits.len();
            while tick_epoch <= epoch_max {
                // Map tick to x position: find nearest commit by epoch
                let nearest_idx = commits.partition_point(|c| c.epoch < tick_epoch);
                let idx = nearest_idx.min(n_commits - 1);
                let x_frac = if n_commits > 1 {
                    idx as f32 / (n_commits - 1) as f32
                } else {
                    0.0
                };
                let tick_x = tick_rect.left() + x_frac * total_width;

                // Draw tick line
                painter.line_segment(
                    [egui::pos2(tick_x, tick_rect.top()), egui::pos2(tick_x, tick_rect.bottom())],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(150, 150, 150, 100)),
                );

                // Draw tick label
                let label = format_epoch_short(tick_epoch, span);
                painter.text(
                    egui::pos2(tick_x + 2.0, tick_rect.top() + 1.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::monospace(8.0),
                    egui::Color32::from_rgba_unmultiplied(180, 180, 180, 200),
                );

                tick_epoch += tick_interval;
            }
        }
    }

    // ── Row 2: Milestones (16px) — only if > 1 milestone ─────────────────

    if milestones.len() > 1 {
        let ms_height = 16.0;
        let ms_rects = equal_segment_rects(
            egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(total_width, ms_height),
            ),
            milestones.len(),
        );
        let (ms_bar, _) = ui.allocate_exact_size(
            egui::vec2(total_width, ms_height),
            egui::Sense::hover(),
        );
        let painter = ui.painter();

        for (i, (ms, seg_rect)) in milestones.iter().zip(ms_rects.iter()).enumerate() {
            let seg_rect = egui::Rect::from_min_size(
                egui::pos2(ms_bar.left() + (seg_rect.left() - 0.0), ms_bar.top()),
                seg_rect.size(),
            );
            let fill = egui::Color32::from_rgb(50, 55, 65);
            painter.rect_filled(seg_rect, 2.0, fill);
            painter.text(
                seg_rect.center(),
                egui::Align2::CENTER_CENTER,
                &ms.name,
                egui::FontId::monospace(9.0),
                egui::Color32::WHITE,
            );

            let seg_id = ui.id().with(("ms_seg", i));
            let resp = ui.interact(seg_rect, seg_id, egui::Sense::click());
            let epoch_start = ms.phase_indices.first()
                .and_then(|&pi| report.phases.get(pi))
                .and_then(|_| commits.first())
                .map(|c| c.epoch)
                .unwrap_or(0);
            let was_clicked = resp.clicked();
            let tooltip = format!("{}: {} phases", ms.name, ms.phase_indices.len());
            resp.on_hover_text(egui::RichText::new(tooltip).monospace().size(10.0));
            if was_clicked {
                let sel = TimelineSelection {
                    kind: TimelineSelectionKind::Milestone,
                    index: i,
                    epoch_start,
                };
                if matches!(&current_selection, Some(s) if s.kind == TimelineSelectionKind::Milestone && s.index == i) {
                    new_selection = Some(None); // deselect
                } else {
                    new_selection = Some(Some(sel));
                }
            }
        }
    }

    // ── Row 3: Phases (18px) ─────────────────────────────────────────────

    {
        let phase_height = 18.0;
        // Determine which phases to show based on selection
        let visible_phase_indices: Vec<usize> = if let Some(TimelineSelection {
            kind: TimelineSelectionKind::Milestone,
            index: ms_idx,
            ..
        }) = &current_selection {
            milestones.get(*ms_idx)
                .map(|ms| ms.phase_indices.clone())
                .unwrap_or_else(|| (0..report.phases.len()).collect())
        } else {
            (0..report.phases.len()).collect()
        };

        let visible_count = visible_phase_indices.len();
        if visible_count > 0 {
            let (phase_bar, _) = ui.allocate_exact_size(
                egui::vec2(total_width, phase_height),
                egui::Sense::hover(),
            );
            let seg_rects = equal_segment_rects(phase_bar, visible_count);
            let painter = ui.painter();

            for (seg_pos, &phase_idx) in visible_phase_indices.iter().enumerate() {
                let phase = match report.phases.get(phase_idx) {
                    Some(p) => p,
                    None => continue,
                };
                let seg_rect = seg_rects[seg_pos];

                // Fill
                let fill = gsd_phase_color(phase.status);
                painter.rect_filled(seg_rect, 0.0, fill);

                // Selected border
                let is_selected = matches!(&current_selection, Some(s) if s.kind == TimelineSelectionKind::Phase && s.index == phase_idx);
                if is_selected {
                    painter.rect_stroke(
                        seg_rect,
                        0.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 80)),
                        egui::StrokeKind::Inside,
                    );
                }

                // Label
                let seg_w = seg_rect.width();
                if seg_w >= 60.0 {
                    let label = format!("P{}", phase.number);
                    painter.text(
                        egui::pos2(seg_rect.left() + 4.0, seg_rect.center().y - 3.0),
                        egui::Align2::LEFT_CENTER,
                        &label,
                        egui::FontId::monospace(9.0),
                        egui::Color32::WHITE,
                    );
                    if seg_w >= 100.0 {
                        let name_text: String = phase.name.chars().take(12).collect();
                        let name_text = if phase.name.chars().count() > 12 {
                            format!("{}..", name_text)
                        } else {
                            name_text
                        };
                        painter.text(
                            egui::pos2(seg_rect.left() + 4.0, seg_rect.center().y + 5.0),
                            egui::Align2::LEFT_CENTER,
                            name_text,
                            egui::FontId::monospace(8.0),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                        );
                    }
                }

                // Interaction
                let seg_id = ui.id().with(("phase_seg", phase_idx));
                let resp = ui.interact(seg_rect, seg_id, egui::Sense::click());

                if resp.clicked() {
                    if is_selected {
                        new_selection = Some(None); // deselect on re-click
                    } else {
                        let epoch_start = commits.iter()
                            .find(|c| c.phase_idx == Some(phase_idx))
                            .map(|c| c.epoch)
                            .unwrap_or(0);
                        new_selection = Some(Some(TimelineSelection {
                            kind: TimelineSelectionKind::Phase,
                            index: phase_idx,
                            epoch_start,
                        }));
                    }
                }

                let goal_preview: String = phase.goal.chars().take(80).collect();
                let tooltip = format!(
                    "Phase {}: {}\nGoal: {}\nStatus: {}\nFiles: {}",
                    phase.number, phase.name, goal_preview,
                    phase.status.label(), phase.files.len(),
                );
                resp.on_hover_text(egui::RichText::new(tooltip).monospace().size(10.0));
            }
        }
    }

    // ── Row 4: Commits (14px) ─────────────────────────────────────────────

    {
        const MIN_SEG_WIDTH: f32 = 7.0;
        let commit_height = 14.0;

        // Filter commits based on selection
        let visible_commits: Vec<(usize, &crate::core::pmat_types::CommitSummary)> =
            match &current_selection {
                Some(TimelineSelection { kind: TimelineSelectionKind::Phase, index, .. }) => {
                    // Phase selected: show only that phase's commits
                    commits.iter().enumerate()
                        .filter(|(_, c)| c.phase_idx == Some(*index))
                        .collect()
                }
                Some(TimelineSelection { kind: TimelineSelectionKind::Commit, index, .. }) => {
                    // Commit selected: keep showing the same phase's commits (if the commit has a phase),
                    // otherwise show all commits. This prevents the bar from jumping to all 200 commits.
                    let phase_of_commit = commits.get(*index).and_then(|c| c.phase_idx);
                    if let Some(phase_idx) = phase_of_commit {
                        commits.iter().enumerate()
                            .filter(|(_, c)| c.phase_idx == Some(phase_idx))
                            .collect()
                    } else {
                        commits.iter().enumerate().collect()
                    }
                }
                _ => commits.iter().enumerate().collect(),
            };

        if !visible_commits.is_empty() {
            // Determine how many commits we can display at MIN_SEG_WIDTH
            let max_displayable = (total_width / MIN_SEG_WIDTH).floor() as usize;
            let show_overflow = visible_commits.len() > max_displayable && max_displayable > 1;
            let display_count = if show_overflow {
                max_displayable - 1 // reserve one slot for "..." overflow
            } else {
                visible_commits.len()
            };
            let overflow_count = visible_commits.len().saturating_sub(display_count);

            let total_segs = if show_overflow { display_count + 1 } else { display_count };

            let (commit_bar, _) = ui.allocate_exact_size(
                egui::vec2(total_width, commit_height),
                egui::Sense::hover(),
            );
            let seg_rects = equal_segment_rects(commit_bar, total_segs);
            let painter = ui.painter();

            for (seg_pos, &(commit_vec_idx, commit)) in visible_commits.iter().take(display_count).enumerate() {
                let seg_rect = seg_rects[seg_pos];

                // Fill — darker for unselected, highlight for selected
                let is_selected = matches!(&current_selection, Some(s) if s.kind == TimelineSelectionKind::Commit && s.index == commit_vec_idx);
                let fill = if is_selected {
                    egui::Color32::from_rgb(100, 160, 220)
                } else {
                    egui::Color32::from_rgb(55, 60, 75)
                };
                painter.rect_filled(seg_rect, 0.0, fill);

                // Label: short_sha only if segment wide enough
                let seg_w = seg_rect.width();
                if seg_w >= 40.0 {
                    painter.text(
                        egui::pos2(seg_rect.left() + 2.0, seg_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &commit.short_sha,
                        egui::FontId::monospace(8.0),
                        egui::Color32::from_rgba_unmultiplied(220, 220, 220, 200),
                    );
                }

                // Interaction
                let seg_id = ui.id().with(("commit_seg", commit_vec_idx));
                let resp = ui.interact(seg_rect, seg_id, egui::Sense::click());

                if resp.clicked() {
                    if is_selected {
                        new_selection = Some(None); // deselect on re-click
                    } else {
                        new_selection = Some(Some(TimelineSelection {
                            kind: TimelineSelectionKind::Commit,
                            index: commit_vec_idx,
                            epoch_start: commit.epoch,
                        }));
                    }
                }

                resp.on_hover_ui(|ui| {
                    let (y, m, d, hh, mm) = crate::core::time_utils::epoch_to_civil(commit.epoch);
                    let tooltip = format!(
                        "{}\nAuthor: {}\nDate: {}-{:02}-{:02} {:02}:{:02}\nFiles: {}",
                        commit.message, commit.author,
                        y, m, d, hh, mm,
                        commit.file_count,
                    );
                    ui.label(egui::RichText::new(tooltip).monospace().size(10.0));
                });
            }

            // Overflow segment
            if show_overflow {
                let seg_rect = seg_rects[display_count];
                painter.rect_filled(seg_rect, 0.0, egui::Color32::from_rgb(70, 70, 80));
                painter.text(
                    seg_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "...",
                    egui::FontId::monospace(8.0),
                    egui::Color32::from_rgba_unmultiplied(200, 200, 200, 200),
                );
                let seg_id = ui.id().with("commit_overflow");
                let resp = ui.interact(seg_rect, seg_id, egui::Sense::hover());
                resp.on_hover_text(
                    egui::RichText::new(format!("{} more commits", overflow_count))
                        .monospace()
                        .size(10.0),
                );
            }
        }
    }

    // ── Pre-compute SHA for mutation (while borrows are still active) ────

    let pending_sha: Option<Option<String>> = if let Some(ref new_sel) = new_selection {
        if *new_sel != *current_selection {
            match new_sel {
                Some(sel) => {
                    let sha_opt: Option<String> = match sel.kind {
                        crate::core::pmat_types::TimelineSelectionKind::Commit => {
                            commits.get(sel.index).map(|c| c.sha.clone())
                        }
                        crate::core::pmat_types::TimelineSelectionKind::Phase => {
                            commits.iter()
                                .find(|c| c.phase_idx == Some(sel.index))
                                .map(|c| c.sha.clone())
                        }
                        crate::core::pmat_types::TimelineSelectionKind::Milestone => {
                            let phase_indices: Vec<usize> = milestones.get(sel.index)
                                .map(|ms| ms.phase_indices.clone())
                                .unwrap_or_default();
                            commits.iter()
                                .find(|c| c.phase_idx.map(|pi| phase_indices.contains(&pi)).unwrap_or(false))
                                .map(|c| c.sha.clone())
                        }
                    };
                    Some(sha_opt)
                }
                None => Some(None), // selection cleared
            }
        } else {
            None // no change
        }
    } else {
        None
    };

    // Release immutable borrows before mutable operations
    let _ = (report, commits, milestones, current_selection);

    // ── Reset button ─────────────────────────────────────────────────────

    draw_timeline_reset_button(ui, state);

    // ── Apply mutations ───────────────────────────────────────────────────

    if let Some(new_sel) = new_selection {
        state.timeline_selection = new_sel;
        if let Some(sha_opt) = pending_sha {
            state.delta_requested = true;
            match sha_opt {
                Some(from_sha) => {
                    // Save current color mode before switching to GitDiff
                    if state.pre_timeline_color_mode.is_none() {
                        state.pre_timeline_color_mode = Some(state.color_mode);
                    }
                    state.color_mode = ColorMode::GitDiff;
                    state.git_diff_window = crate::metrics::evo::git_walker::DiffWindow::CommitRange {
                        from: from_sha,
                        to: "HEAD".to_string(),
                    };
                    state.git_diff_requested = true;
                }
                None => {
                    // Selection cleared — restore previous color mode
                    if let Some(prev) = state.pre_timeline_color_mode.take() {
                        state.color_mode = prev;
                    }
                    state.git_diff_window = crate::metrics::evo::git_walker::DiffWindow::default();
                    state.git_diff_requested = true;
                }
            }
        }
    }
}

/// Draw a reset button that clears the timeline selection when one is active.
///
/// Only visible when `state.timeline_selection.is_some()`. On click: clears
/// selection, delta report, and delta_requested flag.
pub(crate) fn draw_timeline_reset_button(ui: &mut egui::Ui, state: &mut crate::app::state::AppState) {
    if state.timeline_selection.is_none() {
        return;
    }
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        let btn = egui::Button::new(
            egui::RichText::new("x  Reset filter")
                .small()
                .color(egui::Color32::from_rgb(220, 180, 80)),
        )
        .small()
        .fill(egui::Color32::from_rgba_unmultiplied(60, 50, 30, 200));
        if ui.add(btn).on_hover_text("Clear timeline selection").clicked() {
            state.timeline_selection = None;
            state.timeline_delta_report = None;
            state.delta_requested = false;
            // Restore previous color mode
            if let Some(prev) = state.pre_timeline_color_mode.take() {
                state.color_mode = prev;
            }
            // Restore default git diff window on reset
            state.git_diff_window = crate::metrics::evo::git_walker::DiffWindow::default();
            state.git_diff_requested = true;
        }
    });
}

// ── Timeline navigator helpers ──────────────────────────────────────────────

/// Choose a tick granularity in seconds for the given epoch span.
///
/// Returns a tick interval such that ~5-10 ticks appear across the span.
/// Matches: 0 → 60s, ≤2h → 600s (10min), ≤2d → 14400s (4h),
///          ≤60d → 86400s (1d), otherwise → 2592000s (30d/month).
fn choose_tick_granularity_secs(span_secs: i64) -> i64 {
    if span_secs <= 0 {
        60
    } else if span_secs <= 2 * 3600 {
        // Up to 2 hours → 10-minute ticks
        600
    } else if span_secs <= 2 * 86400 {
        // Up to 2 days → 4-hour ticks
        14400
    } else if span_secs <= 60 * 86400 {
        // Up to 60 days → daily ticks
        86400
    } else {
        // Longer → monthly ticks (30-day approximation)
        2_592_000
    }
}

/// Divide `bar_rect` into `count` equal-width sub-rects horizontally.
///
/// Returns an empty vec if count is 0; the full rect for count=1.
fn equal_segment_rects(bar_rect: egui::Rect, count: usize) -> Vec<egui::Rect> {
    if count == 0 {
        return Vec::new();
    }
    let seg_w = bar_rect.width() / count as f32;
    (0..count)
        .map(|i| {
            egui::Rect::from_min_size(
                egui::pos2(bar_rect.left() + i as f32 * seg_w, bar_rect.top()),
                egui::vec2(seg_w, bar_rect.height()),
            )
        })
        .collect()
}

/// Format an epoch as a compact, readable string (no chrono dependency).
///
/// - Span < 1 day: shows "HH:MM" (UTC)
/// - Span < 30 days: shows "MMM DD" (e.g. "Mar 15")
/// - Span >= 30 days: shows "YYYY-MM"
fn format_epoch_short(epoch: i64, span_secs: i64) -> String {
    const MONTH_NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let (y, m, d, hh, mm) = crate::core::time_utils::epoch_to_civil(epoch);

    if span_secs < 86400 {
        return format!("{:02}:{:02}", hh, mm);
    }

    let month_name = MONTH_NAMES[((m - 1) as usize).min(11)];

    if span_secs < 30 * 86400 {
        // Show "MMM DD"
        format!("{} {:02}", month_name, d)
    } else {
        // Show "YYYY-MM"
        format!("{}-{:02}", y, m)
    }
}

// ── Pipeline state transition helpers (testable) ────────────────────────────

/// Snapshot pipeline guard: returns true if a snapshot write should be started.
/// Clears `requested`, sets `running`. Caller spawns the thread if returned true.
///
/// Returns false (and does not set running) when already running or no root.
#[allow(dead_code)]
fn snapshot_pipeline_should_start(
    snapshot_write_requested: &mut bool,
    snapshot_write_running: &mut bool,
    has_root: bool,
) -> bool {
    if !*snapshot_write_requested {
        return false;
    }
    *snapshot_write_requested = false;
    if *snapshot_write_running || !has_root {
        return false;
    }
    *snapshot_write_running = true;
    true
}

/// Delta pipeline guard: returns true if a delta compute should be started.
/// Clears `requested`, sets `running`. Caller spawns the thread if returned true.
/// When selection is None, clears `delta_report` and returns false (correct behavior).
///
/// Returns false (and does not set running) when already running, no root, or no selection.
#[allow(dead_code)]
fn delta_pipeline_should_start(
    delta_requested: &mut bool,
    delta_running: &mut bool,
    has_root: bool,
    has_selection: bool,
    delta_report_present: &mut bool,
) -> bool {
    if !*delta_requested {
        return false;
    }
    *delta_requested = false;
    if *delta_running || !has_root {
        return false;
    }
    if !has_selection {
        *delta_report_present = false;
        return false;
    }
    *delta_running = true;
    true
}

#[cfg(test)]
mod timeline_tests {
    use super::*;

    #[test]
    fn test_choose_tick_granularity_1h() {
        // 1-hour span → 10-minute ticks (600s)
        assert_eq!(choose_tick_granularity_secs(3600), 600);
    }

    #[test]
    fn test_choose_tick_granularity_1d() {
        // 1-day span → 4-hour ticks (14400s)
        assert_eq!(choose_tick_granularity_secs(86400), 14400);
    }

    #[test]
    fn test_choose_tick_granularity_30d() {
        // 30-day span → daily ticks (86400s)
        assert_eq!(choose_tick_granularity_secs(86400 * 30), 86400);
    }

    #[test]
    fn test_choose_tick_granularity_1y() {
        // 1-year span → monthly ticks (2592000s = 30 days)
        assert_eq!(choose_tick_granularity_secs(86400 * 365), 2_592_000);
    }

    #[test]
    fn test_choose_tick_granularity_zero() {
        // Zero span → fallback (60s)
        assert_eq!(choose_tick_granularity_secs(0), 60);
    }

    #[test]
    fn test_equal_segment_rects_even() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 10.0));
        let segs = equal_segment_rects(bar, 4);
        assert_eq!(segs.len(), 4);
        // Each segment should be 25px wide
        for seg in &segs {
            assert!((seg.width() - 25.0).abs() < 0.5, "expected 25px width, got {}", seg.width());
        }
        // First seg starts at 0, second at 25, etc.
        assert!((segs[0].left() - 0.0).abs() < 0.5);
        assert!((segs[1].left() - 25.0).abs() < 0.5);
        assert!((segs[2].left() - 50.0).abs() < 0.5);
        assert!((segs[3].left() - 75.0).abs() < 0.5);
    }

    #[test]
    fn test_equal_segment_rects_zero() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 10.0));
        let segs = equal_segment_rects(bar, 0);
        assert!(segs.is_empty(), "count=0 should return empty vec");
    }

    #[test]
    fn test_equal_segment_rects_one() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 10.0));
        let segs = equal_segment_rects(bar, 1);
        assert_eq!(segs.len(), 1);
        assert!((segs[0].left() - bar.left()).abs() < 0.5);
        assert!((segs[0].right() - bar.right()).abs() < 0.5);
    }

    // ── Pipeline state transition tests ─────────────────────────────────────

    #[test]
    fn test_snapshot_pipeline_state_transitions() {
        // When requested=true and running=false: should start, transitions to running=true
        let mut requested = true;
        let mut running = false;
        let should_start = snapshot_pipeline_should_start(&mut requested, &mut running, true);
        assert!(should_start, "should start when requested=true, running=false");
        assert!(!requested, "requested flag must be cleared");
        assert!(running, "running flag must be set");

        // Second call with running=true: same request is skipped (no double-spawn)
        let mut requested2 = true;
        let should_start2 = snapshot_pipeline_should_start(&mut requested2, &mut running, true);
        assert!(!should_start2, "should NOT start when running=true");
        assert!(!requested2, "requested flag cleared even when skipped");
        assert!(running, "running stays true");
    }

    #[test]
    fn test_snapshot_pipeline_not_requested() {
        // When requested=false: no transition
        let mut requested = false;
        let mut running = false;
        let should_start = snapshot_pipeline_should_start(&mut requested, &mut running, true);
        assert!(!should_start, "should not start when not requested");
        assert!(!running, "running stays false");
    }

    #[test]
    fn test_delta_pipeline_no_selection() {
        // When delta_requested=true but has_selection=false:
        // delta_report is cleared, delta_running stays false
        let mut delta_requested = true;
        let mut delta_running = false;
        let mut delta_report_present = true; // was Some(...)
        let should_start = delta_pipeline_should_start(
            &mut delta_requested,
            &mut delta_running,
            true, // has_root
            false, // has_selection = None
            &mut delta_report_present,
        );
        assert!(!should_start, "should NOT start without a selection");
        assert!(!delta_requested, "requested cleared");
        assert!(!delta_running, "running stays false when no selection");
        assert!(!delta_report_present, "delta_report cleared when no selection");
    }

    #[test]
    fn test_delta_pipeline_with_selection() {
        // When delta_requested=true and has_selection=true: transitions to running=true
        let mut delta_requested = true;
        let mut delta_running = false;
        let mut delta_report_present = false;
        let should_start = delta_pipeline_should_start(
            &mut delta_requested,
            &mut delta_running,
            true,
            true, // has_selection
            &mut delta_report_present,
        );
        assert!(should_start, "should start with selection");
        assert!(!delta_requested, "requested cleared");
        assert!(delta_running, "running set to true");
    }

    #[test]
    fn test_delta_pipeline_already_running() {
        // When delta_running=true: new request is skipped
        let mut delta_requested = true;
        let mut delta_running = true;
        let mut delta_report_present = false;
        let should_start = delta_pipeline_should_start(
            &mut delta_requested,
            &mut delta_running,
            true,
            true,
            &mut delta_report_present,
        );
        assert!(!should_start, "should NOT start when already running");
        assert!(!delta_requested, "requested cleared even when skipped");
        assert!(delta_running, "running stays true");
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
