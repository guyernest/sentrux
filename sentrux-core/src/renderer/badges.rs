//! Entry-point badge rendering — small colored dots and grade labels on file blocks.
//!
//! Badges are drawn in screen space (fixed pixel size regardless of zoom).
//! High-confidence entry points get the theme's badge_high color; low-confidence
//! get badge_low. Only visible when the file block is large enough on screen.
//! TDG grade badges show the letter grade (A+, B-, etc.) when ColorMode::TdgGrade.

use crate::layout::types::{ColorMode, RectKind, RenderData};
use super::RenderContext;
use egui::{Color32, CornerRadius, Stroke, StrokeKind};

/// Draw entry-point badges at the top-right corner of file rects.
/// Rendered in screen space so they always appear correctly positioned
/// regardless of zoom level. Size is fixed in screen pixels (not world).
pub fn draw_badges(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    rd: &RenderData,
    ctx: &RenderContext,
) {
    let ep_set = build_entry_point_set(ctx);
    if ep_set.is_empty() {
        return;
    }

    let canvas_origin = clip_rect.min;
    let vp = &ctx.viewport;
    let badge_size = 6.0_f32;

    let inset = ctx.settings.file_rect_inset;
    for r in &rd.rects {
        if let Some(confidence) = badge_candidate(r, &ep_set, vp) {
            let screen_rect = vp.world_to_screen_rect(r.x, r.y, r.w, r.h, canvas_origin).shrink(inset);
            if screen_rect.width() >= 14.0 && screen_rect.height() >= 14.0 {
                draw_single_badge(painter, screen_rect, badge_size, confidence, ctx);
            }
        }
    }
}

/// Check if a rect is an entry-point file that is visible. Returns confidence if so.
fn badge_candidate<'a>(
    r: &crate::layout::types::LayoutRectSlim,
    ep_set: &std::collections::HashMap<&str, &'a str>,
    vp: &crate::layout::viewport::ViewportTransform,
) -> Option<&'a str> {
    if r.kind != RectKind::File { return None; }
    let confidence = *ep_set.get(r.path.as_str())?;
    if !vp.is_visible(r.x, r.y, r.w, r.h) { return None; }
    Some(confidence)
}

/// Build a map of entry-point file paths to their confidence level.
fn build_entry_point_set<'a>(ctx: &'a RenderContext) -> std::collections::HashMap<&'a str, &'a str> {
    ctx.snapshot
        .as_ref()
        .map(|snap| {
            snap.entry_points
                .iter()
                .map(|ep| (ep.file.as_str(), ep.confidence.as_str()))
                .collect()
        })
        .unwrap_or_default()
}

/// Draw TDG grade badges on treemap nodes when ColorMode::TdgGrade is active.
///
/// Renders the PMAT letter grade (A+, B-, C, etc.) at the top-left corner of each
/// file rect that is large enough on screen (28px minimum in both axes).
/// Skips rendering entirely if no pmat_report is set on the context.
pub fn draw_tdg_badges(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    rd: &RenderData,
    ctx: &RenderContext,
) {
    // Only draw when TdgGrade mode is active
    if ctx.color_mode != ColorMode::TdgGrade {
        return;
    }
    // Skip if no PMAT report available
    let report = match ctx.pmat_report {
        Some(r) => r,
        None => return,
    };

    let canvas_origin = clip_rect.min;
    let vp = &ctx.viewport;
    let inset = ctx.settings.file_rect_inset;

    for r in &rd.rects {
        if r.kind != RectKind::File {
            continue;
        }
        if !vp.is_visible(r.x, r.y, r.w, r.h) {
            continue;
        }

        let screen_rect = vp.world_to_screen_rect(r.x, r.y, r.w, r.h, canvas_origin).shrink(inset);

        // Respect the 28px threshold — too small to render readable text
        if !should_draw_tdg_badge(screen_rect.width(), screen_rect.height()) {
            continue;
        }

        // Look up the file's PMAT grade
        let idx = match report.by_path.get(r.path.as_str()) {
            Some(&i) => i,
            None => continue,
        };
        let grade_raw = &report.tdg.files[idx].grade;
        let display = crate::core::pmat_types::grade_to_display(grade_raw);

        draw_tdg_grade_text(painter, screen_rect, display);
    }
}

/// Draw the grade text at the top-left of a file rect with a subtle background pill.
fn draw_tdg_grade_text(
    painter: &egui::Painter,
    screen_rect: egui::Rect,
    display: &str,
) {
    let font_size = 9.0_f32;
    let offset_x = 3.0_f32;
    let offset_y = 3.0_f32;

    let text_pos = egui::pos2(screen_rect.left() + offset_x, screen_rect.top() + offset_y);

    // Draw a small semi-transparent dark background pill for readability
    let char_w = font_size * 0.6;
    let text_w = display.len() as f32 * char_w;
    let pill = egui::Rect::from_min_size(
        egui::pos2(text_pos.x - 1.0, text_pos.y - 1.0),
        egui::vec2(text_w + 3.0, font_size + 2.0),
    );
    painter.rect_filled(pill, CornerRadius::same(2), Color32::from_rgba_premultiplied(0, 0, 0, 140));

    // Draw grade text in white
    painter.text(
        text_pos,
        egui::Align2::LEFT_TOP,
        display,
        egui::FontId::monospace(font_size),
        Color32::WHITE,
    );
}

/// Determine whether a badge should be drawn at the given screen dimensions.
/// Minimum 28px in both axes required to avoid text being unreadably small.
pub fn should_draw_tdg_badge(width: f32, height: f32) -> bool {
    width >= 28.0 && height >= 28.0
}

#[cfg(test)]
mod badge_tests {
    use super::*;

    #[test]
    fn tdg_badge_skips_when_width_below_28px() {
        assert!(!should_draw_tdg_badge(27.9, 50.0), "should skip when width < 28px");
    }

    #[test]
    fn tdg_badge_skips_when_height_below_28px() {
        assert!(!should_draw_tdg_badge(50.0, 27.9), "should skip when height < 28px");
    }

    #[test]
    fn tdg_badge_draws_when_both_above_28px() {
        assert!(should_draw_tdg_badge(28.0, 28.0), "should draw when both >= 28px");
        assert!(should_draw_tdg_badge(100.0, 100.0), "should draw for large rects");
    }

    #[test]
    fn tdg_badge_skips_exact_boundary_below() {
        assert!(!should_draw_tdg_badge(27.99, 28.0));
        assert!(!should_draw_tdg_badge(28.0, 27.99));
    }
}

/// Draw a single badge dot at the top-right corner of a screen rect.
fn draw_single_badge(
    painter: &egui::Painter,
    screen_rect: egui::Rect,
    badge_size: f32,
    confidence: &str,
    ctx: &RenderContext,
) {
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(
            screen_rect.right() - badge_size - 2.0,
            screen_rect.top() + 2.0,
        ),
        egui::vec2(badge_size, badge_size),
    );

    let tc = &ctx.theme_config;
    let fill = if confidence == "high" { tc.badge_high } else { tc.badge_low };

    painter.rect_filled(badge_rect, CornerRadius::ZERO, fill);
    painter.rect_stroke(badge_rect, CornerRadius::ZERO, Stroke::new(1.0, tc.section_border), StrokeKind::Middle);
}
