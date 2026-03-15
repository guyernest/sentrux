//! Rectangle rendering — draws file blocks and directory sections.
//!
//! Handles color mode selection, connectivity dimming (unconnected files fade),
//! hover/selection highlighting, header strips for directory sections, and
//! zoom-proportional text labels with monospace font.

use crate::layout::types::{ColorMode, LayoutRectSlim, RectKind, RenderData};
use crate::layout::viewport::ViewportTransform;
use super::colors;
use crate::core::heat;
use super::RenderContext;
use crate::layout::types::EdgeFilter;
use egui::{Color32, CornerRadius, Stroke, StrokeKind};
use std::collections::HashSet;

/// Bundles common drawing parameters to reduce function argument counts.
struct DrawCtx<'a> {
    painter: &'a egui::Painter,
    tc: &'a crate::core::settings::ThemeConfig,
    fs: f32,
    cw: f32,
    px: f32,
    py: f32,
}

/// Draw all layout rectangles of a given kind (file or section) onto the painter.
pub fn draw_rects(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    rd: &RenderData,
    ctx: &RenderContext,
    kind: RectKind,
    lod_full: bool,
) {
    let canvas_origin = clip_rect.min;
    let vp = &ctx.viewport;
    let tc = &ctx.theme_config;

    // Pre-build connectivity set for hover/selected file — dim unconnected files.
    let connected_files: Option<HashSet<&str>> = build_connected_set(rd, ctx, kind, lod_full);

    // ONE global font size for ALL text. Scales with zoom.
    let fs = (ctx.settings.font_scale * 72.0 * vp.scale as f32).clamp(4.0, 40.0);
    let cw = fs * 0.62; // monospace char width
    let px = fs * 0.25;
    let py = fs * 0.15;

    let dctx = DrawCtx { painter, tc, fs, cw, px, py };

    for r in &rd.rects {
        if r.kind != kind {
            continue;
        }

        // Viewport culling
        if !vp.is_visible(r.x, r.y, r.w, r.h) {
            continue;
        }

        let screen_rect = vp.world_to_screen_rect(r.x, r.y, r.w, r.h, canvas_origin);

        // Skip sub-pixel rects
        if screen_rect.width() < 1.0 || screen_rect.height() < 1.0 {
            continue;
        }

        match kind {
            RectKind::Section | RectKind::Root => {
                draw_section_rect(&dctx, screen_rect, r, ctx, vp, lod_full);
            }
            RectKind::File => {
                draw_file_rect(&dctx, screen_rect, r, ctx, &connected_files, lod_full);
            }
        }
    }
}

/// Build set of files connected to the active (hovered/selected) file via edges.
/// Returns None if no file is active or not in file-level full-LOD mode.
///
/// When in Risk mode and community_highlight is set, uses the BFS community
/// set instead of the edge-adjacency spotlight (broader highlight).
fn build_connected_set<'a>(
    rd: &'a RenderData,
    ctx: &'a RenderContext,
    kind: RectKind,
    lod_full: bool,
) -> Option<HashSet<&'a str>> {
    if kind != RectKind::File || !lod_full {
        return None;
    }
    // Risk mode with community highlight: use BFS community set
    if ctx.color_mode == ColorMode::Risk {
        if let Some(community) = ctx.community_highlight {
            let set: HashSet<&str> = community.iter().map(|s| s.as_str()).collect();
            if !set.is_empty() {
                return Some(set);
            }
        }
    }
    let active_file = ctx.selected_path.or(ctx.hovered_path);
    active_file.map(|af| {
        let mut set = HashSet::new();
        set.insert(af);
        let adj = &rd.edge_adjacency;
        let edge_type = match ctx.edge_filter {
            EdgeFilter::All => "all",
            EdgeFilter::Imports => "import",
            EdgeFilter::Calls => "call",
            EdgeFilter::Inherit => "inherit",
        };
        for neighbor in adj.connected(af, edge_type) {
            set.insert(neighbor);
        }
        set
    })
}

/// Render a section/root rectangle: background, border, header strip, and label.
fn draw_section_rect(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    vp: &ViewportTransform,
    lod_full: bool,
) {
    let bg = dctx.tc.section_color(r.depth);
    dctx.painter.rect_filled(screen_rect, CornerRadius::ZERO, bg);

    if screen_rect.width() > 10.0 {
        dctx.painter.rect_stroke(
            screen_rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, dctx.tc.section_border),
            StrokeKind::Middle,
        );
    }

    let strip_h = vp.ws(r.header_h);
    if lod_full && strip_h > 4.0 && screen_rect.width() > 20.0 {
        draw_section_header(dctx, screen_rect, r, ctx, strip_h);
    }
}

/// Render the header strip and label text for a section rectangle.
fn draw_section_header(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    strip_h: f32,
) {
    let strip = egui::Rect::from_min_size(
        screen_rect.left_top(),
        egui::vec2(screen_rect.width(), strip_h),
    );
    dctx.painter.rect_filled(strip, CornerRadius::ZERO, dctx.tc.header_strip_bg);

    if dctx.fs + dctx.py >= strip_h {
        return;
    }

    let label = if r.path.is_empty() || r.path == "/" {
        ctx.root_path.unwrap_or("/").to_string()
    } else {
        let dirname = r.path.rsplit('/').next().unwrap_or(&r.path);
        format!("./{}/", dirname)
    };

    let max_chars = ((screen_rect.width() - dctx.px * 2.0) / dctx.cw).max(0.0) as usize;
    let display = if max_chars < 3 {
        ""
    } else if label.chars().count() > max_chars {
        &label[..label.floor_char_boundary(max_chars)]
    } else {
        &label
    };
    if !display.is_empty() {
        dctx.painter.text(
            egui::pos2(screen_rect.left() + dctx.px, screen_rect.top() + dctx.py),
            egui::Align2::LEFT_TOP,
            display,
            egui::FontId::monospace(dctx.fs),
            dctx.tc.section_label,
        );
    }
}

/// Compute the final display color for a file rect, applying spotlight dimming.
fn file_display_color(
    ctx: &RenderContext,
    path: &str,
    connected_files: &Option<HashSet<&str>>,
    lod_full: bool,
) -> Color32 {
    let base_color = file_color(ctx, path);
    if !lod_full {
        return base_color;
    }
    let has_spotlight = connected_files.is_some();
    let is_spotlit = connected_files.as_ref().is_some_and(|c| c.contains(path));
    if is_spotlit {
        if ctx.color_mode == ColorMode::Monochrome {
            ctx.theme_config.file_surface_spotlit
        } else {
            // Blend toward white by a fraction to brighten without hue shift.
            // factor ~0.25 gives a visible but subtle lift.
            let [r, g, b, _] = base_color.to_array();
            let factor = 0.25_f32;
            Color32::from_rgb(
                (r as f32 + (255.0 - r as f32) * factor) as u8,
                (g as f32 + (255.0 - g as f32) * factor) as u8,
                (b as f32 + (255.0 - b as f32) * factor) as u8,
            )
        }
    } else if has_spotlight {
        // Dim unconnected files: halve RGB, keep fully opaque (no alpha double-dim)
        let [r, g, b, _] = base_color.to_array();
        Color32::from_rgb(r / 2, g / 2, b / 2)
    } else {
        base_color
    }
}

/// Render a file rectangle: fill, border, hover/selected highlights, and text.
fn draw_file_rect(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    connected_files: &Option<HashSet<&str>>,
    lod_full: bool,
) {
    let color = file_display_color(ctx, &r.path, connected_files, lod_full);
    let s = &ctx.settings;
    let inset_rect = screen_rect.shrink(s.file_rect_inset);
    dctx.painter.rect_filled(inset_rect, CornerRadius::ZERO, color);

    if lod_full {
        draw_file_borders(&dctx, screen_rect, inset_rect, r, ctx);

        if inset_rect.width() > dctx.cw * 2.0 && inset_rect.height() > dctx.fs + dctx.py * 2.0 {
            draw_file_text(dctx, inset_rect, r, ctx);
        }
    }
}

/// Draw border, hover highlight, and selected highlight for a file rect.
fn draw_file_borders(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    inset_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
) {
    dctx.painter.rect_stroke(
        inset_rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, dctx.tc.file_border),
        StrokeKind::Middle,
    );

    if ctx.hovered_path == Some(r.path.as_str()) {
        dctx.painter.rect_stroke(
            screen_rect, CornerRadius::ZERO,
            Stroke::new(1.0, dctx.tc.hover_stroke),
            StrokeKind::Outside,
        );
    }

    if ctx.selected_path == Some(r.path.as_str()) {
        dctx.painter.rect_stroke(
            screen_rect, CornerRadius::ZERO,
            Stroke::new(2.0, dctx.tc.selected_stroke),
            StrokeKind::Outside,
        );
    }
}

/// Draw file name and stats line text inside a file rect.
fn draw_file_text(
    dctx: &DrawCtx,
    inset_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
) {
    let name = r.path.rsplit('/').next().unwrap_or(&r.path);
    let display_name = truncate_to_fit(name, inset_rect.width(), dctx.cw, dctx.px, 2);

    if display_name.is_empty() {
        return;
    }

    let text_x = inset_rect.left() + dctx.px;
    let text_y = inset_rect.top() + dctx.py;
    let name_bottom = dctx.painter.text(
        egui::pos2(text_x, text_y),
        egui::Align2::LEFT_TOP,
        display_name,
        egui::FontId::monospace(dctx.fs),
        dctx.tc.file_label,
    ).max.y;

    draw_stats_line(dctx, inset_rect, r, ctx, text_x, name_bottom);
}

/// Truncate a string to fit within `width` given padding and char width.
/// Returns empty str if fewer than `min_chars` fit.
fn truncate_to_fit(s: &str, width: f32, cw: f32, px: f32, min_chars: usize) -> &str {
    let max_chars = ((width - px * 2.0) / cw).max(0.0) as usize;
    if max_chars < min_chars {
        ""
    } else if s.chars().count() > max_chars {
        &s[..s.floor_char_boundary(max_chars)]
    } else {
        s
    }
}

/// Draw the stats line below the file name if there is room.
fn draw_stats_line(
    dctx: &DrawCtx,
    inset_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    text_x: f32,
    name_bottom: f32,
) {
    let gap = dctx.fs * 0.1;
    if name_bottom + gap + dctx.fs >= inset_rect.bottom() - dctx.py {
        return;
    }
    if let Some(entry) = ctx.file_index.get(r.path.as_str()) {
        let sl = &entry.stats_line;
        let stat_display = truncate_to_fit(sl.as_str(), inset_rect.width(), dctx.cw, dctx.px, 0);
        dctx.painter.text(
            egui::pos2(text_x, name_bottom + gap),
            egui::Align2::LEFT_TOP,
            stat_display,
            egui::FontId::monospace(dctx.fs),
            dctx.tc.text_secondary,
        );
    }
}

/// Compute file color based on current color mode. Used by both main canvas and minimap.
pub fn file_color(ctx: &RenderContext, path: &str) -> Color32 {
    match ctx.color_mode {
        ColorMode::Monochrome => ctx.theme_config.file_surface,
        ColorMode::Language => color_by_language(ctx, path),
        ColorMode::Heat => color_by_heat(ctx, path),
        ColorMode::Git => color_by_git(ctx, path),
        ColorMode::TdgGrade => color_by_tdg_grade(ctx, path),
        ColorMode::Coverage => color_by_coverage(ctx, path),
        ColorMode::Risk => color_by_risk(ctx, path),
    }
}

/// Coverage color mode: look up per-file line coverage percentage, return blue-to-green gradient.
/// Falls back to monochrome when no coverage report is available.
/// Returns muted gray for files not present in coverage data (not instrumented).
fn color_by_coverage(ctx: &RenderContext, path: &str) -> Color32 {
    let report = match ctx.coverage_report {
        Some(r) => r,
        None => return ctx.theme_config.file_surface,
    };
    let idx = match report.by_path.get(path) {
        Some(&i) => i,
        // File not in coverage data — not instrumented (test files, build scripts, etc.)
        None => return Color32::from_rgb(80, 80, 80),
    };
    let pct = report.files[idx].summary.lines.percent;
    colors::coverage_color(pct)
}

/// Risk color mode: combines PageRank + coverage + clippy warnings into a risk score.
/// Falls back gracefully when individual data sources are missing.
/// Normalizes against the project-level maximum raw risk so the riskiest file is always red.
fn color_by_risk(ctx: &RenderContext, path: &str) -> Color32 {
    let max_raw = ctx.max_risk_raw;

    // Extract basename for graph-metrics lookup (nodes indexed by filename, not full path)
    let basename = path.rsplit('/').next().unwrap_or(path);

    let pagerank = ctx.graph_metrics_report
        .and_then(|gm| gm.by_filename.get(basename).map(|&idx| gm.data.nodes[idx].pagerank))
        .unwrap_or(0.0);

    let coverage_pct = ctx.coverage_report
        .and_then(|cov| cov.by_path.get(path).map(|&idx| cov.files[idx].summary.lines.percent));

    let clippy_count = ctx.clippy_report
        .and_then(|r| r.by_file.get(path))
        .map(|d| d.total);

    colors::risk_color(Some(pagerank), coverage_pct, clippy_count, max_raw)
}

/// Compute the maximum raw risk value across all files in graph-metrics.
/// Used to normalize risk coloring so the riskiest file is always maximally red.
/// Called once when reports change (not per-frame).
pub fn compute_max_risk_raw(
    gm: Option<&crate::core::pmat_types::GraphMetricsReport>,
    cov: Option<&crate::core::pmat_types::CoverageReport>,
    clippy: Option<&crate::core::pmat_types::ClippyReport>,
) -> f64 {
    let gm = match gm {
        Some(gm) => gm,
        None => return 1.0,
    };
    let mut max = 0.0_f64;
    for node in &gm.data.nodes {
        // Use consistent basename strategy matching color_by_risk
        let basename = &node.name;

        let coverage_pct = cov
            .and_then(|c| c.by_basename.get(basename.as_str()).map(|&idx| c.files[idx].summary.lines.percent))
            .unwrap_or(50.0);

        let clippy_count = clippy
            .and_then(|r| r.by_basename.get(basename.as_str()))
            .map(|d| d.total)
            .unwrap_or(0);

        let raw = colors::compute_raw_risk(node.pagerank, coverage_pct, clippy_count);
        if raw > max { max = raw; }
    }
    if max <= 0.0 { 1.0 } else { max }
}

fn color_by_language(ctx: &RenderContext, path: &str) -> Color32 {
    let lang = ctx
        .file_index
        .get(path)
        .map(|e| e.lang.as_str())
        .unwrap_or("unknown");
    colors::language_color(lang)
}

fn color_by_heat(ctx: &RenderContext, path: &str) -> Color32 {
    let h = ctx.heat.get_heat(path, ctx.frame_instant, ctx.settings.heat_half_life);
    if h > 0.01 {
        heat::heat_color(h)
    } else {
        Color32::from_rgb(50, 50, 55)
    }
}

fn color_by_git(ctx: &RenderContext, path: &str) -> Color32 {
    let gs = ctx
        .file_index
        .get(path)
        .map(|e| e.gs.as_str())
        .unwrap_or("");
    colors::git_color(gs)
}

/// TDG grade color mode: look up file grade from pmat_report, return green-to-red gradient.
/// Falls back to theme's file_surface if no report is available.
fn color_by_tdg_grade(ctx: &RenderContext, path: &str) -> Color32 {
    let report = match ctx.pmat_report {
        Some(r) => r,
        None => return ctx.theme_config.file_surface,
    };
    let idx = match report.by_path.get(path) {
        Some(&i) => i,
        None => return ctx.theme_config.file_surface,
    };
    let grade = &report.tdg.files[idx].grade;
    colors::tdg_grade_color(grade)
}
