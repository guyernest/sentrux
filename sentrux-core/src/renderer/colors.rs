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

/// Muted gray for files with no data in the active color mode (unchanged, uninstrumented, etc.).
pub const NO_DATA_GRAY: Color32 = Color32::from_rgb(50, 52, 55);

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

/// Map a normalized git diff intensity (0.0–1.0) to a Color32 gradient.
///
/// - t=0.0: cool blue (30, 107, 155) — unchanged / low activity
/// - t=1.0: hot orange (232, 106, 17) — heavily changed
///
/// Uses linear RGB interpolation with clamping. Visually distinct from
/// the green-to-red quality gradients (Coverage, Risk, TdgGrade).
pub fn git_diff_intensity_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let r = (30.0 + t * (232.0 - 30.0)) as u8;
    let g = (107.0 + t * (106.0 - 107.0)) as u8;
    let b = (155.0 + t * (17.0 - 155.0)) as u8;
    Color32::from_rgb(r, g, b)
}

/// Distinct teal color for files newly created within the diff window.
///
/// Chosen to be visually separate from both the blue (low activity) and
/// orange (high activity) endpoints of the intensity gradient.
pub fn git_diff_new_file_color() -> Color32 {
    Color32::from_rgb(32, 190, 165)
}

/// Compute raw risk value from individual signals.
///
/// Formula: `pagerank * complexity_penalty * (1 - coverage_pct/100) * (1 + ln(clippy_count+1)/5)`
///
/// `complexity_penalty` is derived from the TDG grade: `1.0 - grade_to_t(grade) as f64`.
/// A+ grade yields `penalty=0.0` (near-zero risk for trivially simple hub files like mod.rs).
/// F grade yields `penalty=1.0` (identity — same as the old 3-arg formula).
/// Unknown grade defaults to `penalty=1.0` (conservative — full weight retained).
///
/// Used by both `risk_color` (per-file) and `compute_max_risk_raw` (normalization).
pub fn compute_raw_risk(pagerank: f64, coverage_pct: f64, clippy_count: u32, complexity_penalty: f64) -> f64 {
    let pr = pagerank.clamp(0.0, 1.0);
    let penalty = complexity_penalty.clamp(0.0, 1.0);
    let uncovered = 1.0 - coverage_pct.clamp(0.0, 100.0) / 100.0;
    let lint_factor = 1.0 + (clippy_count as f64 + 1.0).ln() / 5.0;
    pr * penalty * uncovered * lint_factor
}

/// Combine PageRank, coverage, clippy warning count, and TDG complexity penalty into a risk color.
///
/// `complexity_penalty` = `1.0 - grade_to_t(tdg_grade) as f64`. A+ → 0.0 (near-zero risk),
/// F → 1.0 (full weight, same as old formula), unknown → 1.0 (conservative).
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
    complexity_penalty: f64,
) -> Color32 {
    let raw = compute_raw_risk(
        pagerank.unwrap_or(0.0),
        coverage_pct.unwrap_or(50.0),
        clippy_count.unwrap_or(0),
        complexity_penalty,
    );
    let norm = if max_raw <= 0.0 { 1.0 } else { max_raw };
    let t = (raw / norm).clamp(0.0, 1.0) as f32;
    // cool (low risk = green) → hot (high risk = red)
    let r = (30.0 + t * 225.0) as u8;
    let g = (180.0 * (1.0 - t)) as u8;
    let b = 40_u8;
    Color32::from_rgb(r, g, b)
}

/// Map a GSD phase status to a Color32.
///
/// - `Completed`: muted green (phase delivered)
/// - `InProgress`: amber/goldenrod (phase currently active)
/// - `Planned`: steel blue (phase not yet started)
pub fn gsd_phase_color(status: crate::core::pmat_types::PhaseStatus) -> Color32 {
    use crate::core::pmat_types::PhaseStatus;
    match status {
        PhaseStatus::Completed  => Color32::from_rgb(76, 153, 76),   // muted green
        PhaseStatus::InProgress => Color32::from_rgb(220, 165, 32),  // amber/goldenrod
        PhaseStatus::Planned    => Color32::from_rgb(70, 130, 180),  // steel blue
    }
}

#[cfg(test)]
mod gsd_phase_color_tests {
    use super::*;
    use crate::core::pmat_types::PhaseStatus;

    #[test]
    fn gsd_phase_color_completed_is_greenish() {
        let c = gsd_phase_color(PhaseStatus::Completed);
        let [r, g, _b, _] = c.to_array();
        assert!(g > r, "Completed phase should be greenish: r={} g={}", r, g);
    }

    #[test]
    fn gsd_phase_color_in_progress_is_amber() {
        let c = gsd_phase_color(PhaseStatus::InProgress);
        let [r, g, b, _] = c.to_array();
        assert!(r > b, "InProgress phase amber should have r > b: r={} b={}", r, b);
        assert!(g > b, "InProgress phase amber should have g > b: g={} b={}", g, b);
    }

    #[test]
    fn gsd_phase_color_planned_is_bluish() {
        let c = gsd_phase_color(PhaseStatus::Planned);
        let [r, _g, b, _] = c.to_array();
        assert!(b > r, "Planned phase should be bluish: r={} b={}", r, b);
    }
}

#[cfg(test)]
mod git_diff_color_tests {
    use super::*;

    #[test]
    fn git_diff_intensity_color_zero_is_blue() {
        let c = git_diff_intensity_color(0.0);
        let [r, _g, b, _] = c.to_array();
        assert!(r < 60, "t=0 should be blue-ish: r={} (expected < 60)", r);
        assert!(b > 100, "t=0 should be blue-ish: b={} (expected > 100)", b);
    }

    #[test]
    fn git_diff_intensity_color_one_is_orange() {
        let c = git_diff_intensity_color(1.0);
        let [r, _g, b, _] = c.to_array();
        assert!(r > 200, "t=1 should be orange-ish: r={} (expected > 200)", r);
        assert!(b < 30, "t=1 should be orange-ish: b={} (expected < 30)", b);
    }

    #[test]
    fn git_diff_new_file_color_is_teal() {
        let c = git_diff_new_file_color();
        let [r, g, b, _] = c.to_array();
        // Teal: green-dominant with significant blue, not orange
        assert!(g > r, "teal: g({}) should > r({})", g, r);
        assert!(b > r, "teal: b({}) should > r({})", b, r);
    }

    #[test]
    fn git_diff_intensity_color_clamps_out_of_range() {
        // Should not panic for values outside 0..1
        let _ = git_diff_intensity_color(-1.0);
        let _ = git_diff_intensity_color(2.0);
    }

    #[test]
    fn git_diff_intensity_color_midpoint_is_between() {
        let c0 = git_diff_intensity_color(0.0);
        let c1 = git_diff_intensity_color(1.0);
        let cm = git_diff_intensity_color(0.5);
        let [r0, _, b0, _] = c0.to_array();
        let [r1, _, b1, _] = c1.to_array();
        let [rm, _, bm, _] = cm.to_array();
        // Midpoint r should be between r0 and r1
        assert!(rm > r0 && rm < r1, "midpoint r={} should be between {}..{}", rm, r0, r1);
        // Midpoint b should be between b1 and b0 (b decreases from 0→1)
        assert!(bm > b1 && bm < b0, "midpoint b={} should be between {}..{}", bm, b1, b0);
    }
}

#[cfg(test)]
mod compute_raw_risk_tests {
    use super::*;

    /// A+ grade → complexity_penalty = 0.0 → risk should be near-zero (essentially 0).
    #[test]
    fn compute_raw_risk_aplus_near_zero() {
        // penalty=0.0 means A+ grade (1.0 - grade_to_t("APLus") as f64 = 0.0)
        let result = compute_raw_risk(0.9, 50.0, 5, 0.0);
        assert!(result < 0.001, "A+ hub file risk should be near-zero, got: {}", result);
    }

    /// F grade → complexity_penalty = 1.0 → same as old 3-arg formula (identity).
    #[test]
    fn compute_raw_risk_f_grade_full_penalty() {
        // penalty=1.0 means F grade: formula is pr * 1.0 * uncovered * lint_factor
        let result = compute_raw_risk(0.9, 50.0, 5, 1.0);
        // Old 3-arg formula result (manually computed):
        // pr=0.9, uncovered=(1-0.5)=0.5, lint_factor=1+ln(6)/5=1+1.7918/5≈1.3584
        // raw = 0.9 * 0.5 * 1.3584 ≈ 0.6113
        let expected = {
            let pr = 0.9_f64;
            let uncovered = 1.0 - 50.0_f64 / 100.0;
            let lint_factor = 1.0 + (5_f64 + 1.0).ln() / 5.0;
            pr * uncovered * lint_factor
        };
        assert!((result - expected).abs() < 1e-10,
            "F grade should match old formula: got={} expected={}", result, expected);
    }

    /// Unknown grade → complexity_penalty = 1.0 (conservative, same as F).
    /// Higher penalty should yield higher (or equal) risk than lower penalty.
    #[test]
    fn compute_raw_risk_unknown_conservative() {
        let full_penalty = compute_raw_risk(0.9, 50.0, 5, 1.0);
        let half_penalty = compute_raw_risk(0.9, 50.0, 5, 0.5);
        assert!(full_penalty >= half_penalty,
            "Full penalty should be >= half penalty: full={} half={}", full_penalty, half_penalty);
    }
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
        // High pagerank + low coverage + many lints + F grade (penalty=1.0) = high risk = reddish
        let c = risk_color(Some(0.9), Some(10.0), Some(50), 1.0, 1.0);
        let [r, g, _b, _] = c.to_array();
        assert!(r > g, "high risk should be reddish: r={} g={}", r, g);
    }

    #[test]
    fn risk_color_low_risk_is_green() {
        // Low pagerank + high coverage + no lints + A+ grade (penalty=0.0) = low risk = greenish
        let c = risk_color(Some(0.01), Some(95.0), Some(0), 1.0, 0.0);
        let [r, g, _b, _] = c.to_array();
        assert!(g > r, "low risk should be greenish: r={} g={}", r, g);
    }

    #[test]
    fn risk_color_none_inputs_no_panic() {
        // All None should not panic and return a valid color
        let c = risk_color(None, None, None, 1.0, 1.0);
        let [_r, _g, _b, _a] = c.to_array();
        // Just checking it runs without panic
    }
}

