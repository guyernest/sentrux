//! Color mapping functions for all ColorMode variants.
//!
//! Maps file attributes (language, git status, age, blast radius, churn)
//! to `Color32` values. Palette is desaturated for readability — colors
//! distinguish categories without competing with text labels or edges.

use egui::Color32;

/// Shared status colors — used for pass/warn/fail indicators across panels.
pub const STATUS_PASS: Color32 = Color32::from_rgb(72, 191, 145);
pub const STATUS_WARN: Color32 = Color32::from_rgb(255, 193, 7);
pub const STATUS_FAIL: Color32 = Color32::from_rgb(244, 67, 54);

/// Language → color mapping via O(1) match.
pub fn language_color(lang: &str) -> Color32 {
    let (r, g, b) = match lang {
        "python"     => (65, 105, 145),
        "javascript" | "jsx" => (175, 165, 85),
        "typescript" | "tsx" => (60, 110, 168),
        "rust"       => (175, 135, 110),
        "go"         => (55, 140, 165),
        "c"          => (90, 95, 100),
        "cpp"        => (55, 90, 140),
        "java"       => (150, 110, 55),
        "ruby"       => (160, 65, 60),
        "csharp"     => (105, 60, 120),
        "php"        => (105, 110, 150),
        "bash"       => (110, 160, 80),
        "html"       => (175, 80, 55),
        "css"        => (85, 70, 120),
        "scss"       => (155, 95, 125),
        "swift"      => (180, 80, 60),
        "kotlin"     => (135, 105, 190),
        "lua"        => (50, 55, 120),
        "scala"      => (155, 60, 75),
        "elixir"     => (100, 75, 120),
        "haskell"    => (90, 80, 125),
        "zig"        => (180, 135, 60),
        "r"          => (50, 120, 175),
        "dockerfile" => (60, 80, 90),
        "ocaml"      => (180, 110, 45),
        "json"       => (60, 65, 70),
        "toml"       => (130, 75, 50),
        "yaml"       => (155, 50, 55),
        "markdown"   => (50, 70, 135),
        _            => (80, 85, 90),
    };
    Color32::from_rgb(r, g, b)
}

/// Git status → color
pub fn git_color(gs: &str) -> Color32 {
    match gs {
        "A" => Color32::from_rgb(72, 191, 145),
        "M" => Color32::from_rgb(255, 193, 7),
        "MM" => Color32::from_rgb(255, 152, 0),
        "D" => Color32::from_rgb(244, 67, 54),
        "R" => Color32::from_rgb(156, 39, 176),
        "?" => Color32::from_rgb(120, 120, 120),
        _ => Color32::from_rgb(70, 70, 70),
    }
}

/// TDG grade → green-to-red gradient.
/// A+ (t=1.0) = greenish, F (t=0.0) = reddish.
pub fn tdg_grade_color(grade: &str) -> Color32 {
    let t = crate::core::pmat_types::grade_to_t(grade);
    // green(A+) -> yellow(C) -> red(F)
    let r = (30.0 + (1.0 - t) * 225.0) as u8;
    let g = (180.0 * t) as u8;
    let b = 40_u8;
    Color32::from_rgb(r, g, b)
}

/// Map a line coverage percentage (0.0–100.0) to a Color32 gradient.
///
/// - ≥ 80%: green
/// - 40–80%: yellow blend
/// - < 40%: red
///
/// Uses the same RGB interpolation pattern as `tdg_grade_color`:
/// `t = pct / 100`, r = 30 + (1-t)*225, g = 180*t, b = 40.
pub fn coverage_color(line_pct: f64) -> Color32 {
    let t = (line_pct / 100.0).clamp(0.0, 1.0) as f32;
    let r = (30.0 + (1.0 - t) * 225.0) as u8;
    let g = (180.0 * t) as u8;
    let b = 40_u8;
    Color32::from_rgb(r, g, b)
}

/// Combine PageRank, coverage, and clippy warning count into a risk color.
///
/// Risk formula: `raw = pagerank * (1 - coverage_pct/100) * (1 + ln(clippy_count+1)/5)`
///
/// Normalized using `max_raw` (project-level maximum raw risk). If `max_raw <= 0.0`,
/// defaults to `1.0` to avoid division by zero.
///
/// Color gradient: cool/green (low risk) → hot/red (high risk).
pub fn risk_color(
    pagerank: Option<f64>,
    coverage_pct: Option<f64>,
    clippy_count: Option<u32>,
    max_raw: f64,
) -> Color32 {
    let pr = pagerank.unwrap_or(0.0).clamp(0.0, 1.0);
    let uncovered = 1.0 - coverage_pct.unwrap_or(50.0).clamp(0.0, 100.0) / 100.0;
    let lint_factor = 1.0 + (clippy_count.unwrap_or(0) as f64 + 1.0).ln() / 5.0;
    let raw = pr * uncovered * lint_factor;
    let norm = if max_raw <= 0.0 { 1.0 } else { max_raw };
    let t = (raw / norm).clamp(0.0, 1.0) as f32;
    // cool (low risk = green) → hot (high risk = red)
    let r = (30.0 + t * 225.0) as u8;
    let g = (180.0 * (1.0 - t)) as u8;
    let b = 40_u8;
    Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tdg_grade_color_tests {
    use super::*;

    #[test]
    fn tdg_grade_color_aplus_is_greenish() {
        let c = tdg_grade_color("APLus");
        let [r, g, _b, _] = c.to_array();
        assert!(g > r, "A+ should be greenish: r={} g={}", r, g);
    }

    #[test]
    fn tdg_grade_color_f_is_reddish() {
        let c = tdg_grade_color("F");
        let [r, g, _b, _] = c.to_array();
        assert!(r > g, "F should be reddish: r={} g={}", r, g);
    }

    // ── coverage_color tests ─────────────────────────────────────────────

    #[test]
    fn coverage_color_high_is_greenish() {
        let c = coverage_color(85.0);
        let [r, g, _b, _] = c.to_array();
        assert!(g > r, "85% coverage should be greenish: r={} g={}", r, g);
    }

    #[test]
    fn coverage_color_low_is_reddish() {
        let c = coverage_color(20.0);
        let [r, g, _b, _] = c.to_array();
        assert!(r > g, "20% coverage should be reddish: r={} g={}", r, g);
    }

    #[test]
    fn coverage_color_mid_is_yellowish() {
        let c = coverage_color(50.0);
        let [r, g, _b, _] = c.to_array();
        // At 50%, both r and g should be non-zero (yellowish)
        assert!(r > 30, "50% coverage r should be notable: r={}", r);
        assert!(g > 30, "50% coverage g should be notable: g={}", g);
    }

    // ── risk_color tests ─────────────────────────────────────────────────

    #[test]
    fn risk_color_high_risk_is_red() {
        // High pagerank + low coverage + many lints = high risk = reddish
        let c = risk_color(Some(0.9), Some(10.0), Some(50), 1.0);
        let [r, g, _b, _] = c.to_array();
        assert!(r > g, "high risk should be reddish: r={} g={}", r, g);
    }

    #[test]
    fn risk_color_low_risk_is_green() {
        // Low pagerank + high coverage + no lints = low risk = greenish
        let c = risk_color(Some(0.01), Some(95.0), Some(0), 1.0);
        let [r, g, _b, _] = c.to_array();
        assert!(g > r, "low risk should be greenish: r={} g={}", r, g);
    }

    #[test]
    fn risk_color_none_inputs_no_panic() {
        // All None should not panic and return a valid color
        let c = risk_color(None, None, None, 1.0);
        let [_r, _g, _b, _a] = c.to_array();
        // Just checking it runs without panic
    }
}

