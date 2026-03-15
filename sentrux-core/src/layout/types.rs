//! Layout data types — enums, rects, edges, and render output.
//!
//! Defines the vocabulary shared between the layout engine (treemap/blueprint),
//! edge router, and renderer. Layout produces `RenderData`; renderer consumes it.
//! No egui dependency here — all geometry is in abstract f64 world coordinates.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Color modes for file rects
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    /// Color by programming language (each language gets a unique hue)
    Language,
    /// Color by live edit heat (recently changed files glow warm)
    Heat,
    /// Color by git status (added/modified/deleted/untracked)
    Git,
    /// Color by PMAT TDG grade (A+ = green, F = red)
    TdgGrade,
    /// Color by test coverage percentage (green = well-covered, red = uncovered).
    /// Disabled in toolbar until coverage data is available.
    Coverage,
    /// Color by combined risk signal (PageRank × coverage × clippy warnings).
    /// Hot = architecturally risky and poorly covered; cool = safe and tested.
    Risk,
    /// Color by git diff intensity within a selectable time window.
    /// Blue (recently unchanged) → orange (heavily changed); new files get teal.
    #[serde(rename = "GitDiff")]
    GitDiff,
    /// Terminal pixel monochrome: flat neutral surface color, no per-file coloring.
    /// Style guide §10: "File blocks: one neutral surface color."
    /// Also used as the serde fallback for removed variants (Age, Churn, ExecDepth, BlastRadius).
    /// MUST remain the last variant — #[serde(other)] consumes all unrecognized values.
    #[serde(other)]
    Monochrome,
}

impl ColorMode {
    /// All supported color mode variants.
    pub const ALL: &'static [ColorMode] = &[
        ColorMode::Language,
        ColorMode::Heat,
        ColorMode::Git,
        ColorMode::TdgGrade,
        ColorMode::Coverage,
        ColorMode::Risk,
        ColorMode::GitDiff,
        ColorMode::Monochrome,
    ];

    /// Human-readable display label for this color mode.
    pub fn label(self) -> &'static str {
        match self {
            ColorMode::Language => "Language",
            ColorMode::Heat => "Heat",
            ColorMode::Git => "Git Status",
            ColorMode::TdgGrade => "TDG Grade",
            ColorMode::Coverage => "Coverage",
            ColorMode::Risk => "Risk",
            ColorMode::GitDiff => "Git Diff",
            ColorMode::Monochrome => "Mono",
        }
    }
}

// ─── Focus / filter enums ─────────────────────────────────────
// These live here (not in app::state) because layout is their primary
// consumer — weight.rs uses FocusMode to filter files, and edge routing
// uses EdgeFilter. Keeping them here prevents upward dependencies from
// layout → app.

/// Focus mode — filter which files are visible in layout
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FocusMode {
    /// Show everything
    All,
    /// Show only files matching a directory prefix
    Directory(String),
    /// Show only files matching a language
    Language(String),
    /// Show only entry-point files and their connections
    EntryPoints,
    /// Show only files in the blast radius of a selected file
    ImpactRadius(String),
}

impl FocusMode {
    /// Human-readable display label for this focus mode.
    pub fn label(&self) -> &str {
        match self {
            FocusMode::All => "All Files",
            FocusMode::Directory(_) => "Directory",
            FocusMode::Language(_) => "Language",
            FocusMode::EntryPoints => "Entry Points",
            FocusMode::ImpactRadius(_) => "Impact Radius",
        }
    }

    /// Does this focus mode include the given file path?
    /// `impact_files` is the set of transitively affected files (for ImpactRadius mode).
    pub fn includes_with_impact(
        &self,
        path: &str,
        lang: &str,
        is_entry: bool,
        impact_files: Option<&HashSet<String>>,
    ) -> bool {
        match self {
            FocusMode::All => true,
            FocusMode::Directory(prefix) => {
                path == prefix.as_str()
                    || (path.starts_with(prefix.as_str())
                        && path.as_bytes().get(prefix.len()) == Some(&b'/'))
            }
            FocusMode::Language(l) => lang == l.as_str(),
            FocusMode::EntryPoints => is_entry,
            FocusMode::ImpactRadius(center) => {
                path == center.as_str()
                    || impact_files.is_some_and(|s| s.contains(path))
            }
        }
    }

}

// ── ColorMode tests ────────────────────────────────────────────────────────
#[cfg(test)]
mod color_mode_tests {
    use super::*;

    #[test]
    fn color_mode_all_has_exactly_8_variants() {
        assert_eq!(ColorMode::ALL.len(), 8);
    }

    #[test]
    fn color_mode_all_contains_tdg_grade() {
        assert!(ColorMode::ALL.contains(&ColorMode::TdgGrade));
    }

    #[test]
    fn color_mode_tdg_grade_label() {
        assert_eq!(ColorMode::TdgGrade.label(), "TDG Grade");
    }

    #[test]
    fn color_mode_deserialize_churn_gives_monochrome() {
        let val: ColorMode = serde_json::from_str("\"churn\"").expect("should deserialize");
        assert_eq!(val, ColorMode::Monochrome, "old 'churn' variant should fallback to Monochrome");
    }

    #[test]
    fn color_mode_deserialize_tdggrade() {
        let val: ColorMode = serde_json::from_str("\"tdggrade\"").expect("should deserialize");
        assert_eq!(val, ColorMode::TdgGrade);
    }

    #[test]
    fn color_mode_has_coverage_and_risk() {
        assert!(ColorMode::ALL.contains(&ColorMode::Coverage),
            "ColorMode::ALL should contain Coverage");
        assert!(ColorMode::ALL.contains(&ColorMode::Risk),
            "ColorMode::ALL should contain Risk");
    }

    #[test]
    fn color_mode_coverage_label() {
        assert_eq!(ColorMode::Coverage.label(), "Coverage");
    }

    #[test]
    fn color_mode_risk_label() {
        assert_eq!(ColorMode::Risk.label(), "Risk");
    }

    #[test]
    fn color_mode_coverage_serde() {
        // Round-trip Coverage
        let serialized = serde_json::to_string(&ColorMode::Coverage).expect("serialize");
        assert_eq!(serialized, "\"coverage\"");
        let deserialized: ColorMode = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, ColorMode::Coverage);
        // Round-trip Risk
        let serialized = serde_json::to_string(&ColorMode::Risk).expect("serialize");
        assert_eq!(serialized, "\"risk\"");
        let deserialized: ColorMode = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, ColorMode::Risk);
    }

    #[test]
    fn color_mode_serde_other_fallback() {
        // Unknown strings should still fall back to Monochrome
        let val: ColorMode = serde_json::from_str("\"unknown_future_mode\"").expect("should deserialize");
        assert_eq!(val, ColorMode::Monochrome);
        let val2: ColorMode = serde_json::from_str("\"churn\"").expect("should deserialize");
        assert_eq!(val2, ColorMode::Monochrome);
    }

    #[test]
    fn color_mode_coverage_before_monochrome() {
        // Coverage and Risk must appear before Monochrome in ALL
        let all = ColorMode::ALL;
        let mono_pos = all.iter().position(|&m| m == ColorMode::Monochrome)
            .expect("Monochrome must be in ALL");
        let cov_pos = all.iter().position(|&m| m == ColorMode::Coverage)
            .expect("Coverage must be in ALL");
        let risk_pos = all.iter().position(|&m| m == ColorMode::Risk)
            .expect("Risk must be in ALL");
        assert!(cov_pos < mono_pos, "Coverage must appear before Monochrome in ALL");
        assert!(risk_pos < mono_pos, "Risk must appear before Monochrome in ALL");
    }

    // ── GitDiff ColorMode tests ─────────────────────────────────────────

    #[test]
    fn color_mode_git_diff_exists_in_all() {
        assert!(ColorMode::ALL.contains(&ColorMode::GitDiff),
            "ColorMode::ALL should contain GitDiff");
    }

    #[test]
    fn color_mode_git_diff_before_monochrome() {
        let all = ColorMode::ALL;
        let mono_pos = all.iter().position(|&m| m == ColorMode::Monochrome)
            .expect("Monochrome must be in ALL");
        let gitdiff_pos = all.iter().position(|&m| m == ColorMode::GitDiff)
            .expect("GitDiff must be in ALL");
        assert!(gitdiff_pos < mono_pos, "GitDiff must appear before Monochrome in ALL");
    }

    #[test]
    fn color_mode_git_diff_serializes_to_gitdiff() {
        let serialized = serde_json::to_string(&ColorMode::GitDiff).expect("serialize");
        assert_eq!(serialized, "\"GitDiff\"", "GitDiff should serialize to 'GitDiff'");
    }

    #[test]
    fn color_mode_git_diff_deserializes() {
        let val: ColorMode = serde_json::from_str("\"GitDiff\"").expect("should deserialize");
        assert_eq!(val, ColorMode::GitDiff);
    }

    #[test]
    fn color_mode_git_diff_label() {
        assert_eq!(ColorMode::GitDiff.label(), "Git Diff");
    }

    #[test]
    fn color_mode_all_has_8_variants_with_git_diff() {
        assert_eq!(ColorMode::ALL.len(), 8, "ALL should have 8 variants after adding GitDiff");
    }
}

/// Which edge types to display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeFilter {
    /// Show all edge types
    All,
    /// Show only import/require edges
    Imports,
    /// Show only function call edges
    Calls,
    /// Show only inheritance/implementation edges
    Inherit,
}

impl EdgeFilter {
    /// All supported edge filter variants.
    pub const ALL: &'static [EdgeFilter] = &[
        EdgeFilter::All,
        EdgeFilter::Imports,
        EdgeFilter::Calls,
        EdgeFilter::Inherit,
    ];

    /// Human-readable display label for this edge filter.
    pub fn label(self) -> &'static str {
        match self {
            EdgeFilter::All => "All Edges",
            EdgeFilter::Imports => "Imports",
            EdgeFilter::Calls => "Calls",
            EdgeFilter::Inherit => "Inherit",
        }
    }

    /// Does this filter accept the given edge_type string?
    pub fn accepts(self, edge_type: &str) -> bool {
        match self {
            EdgeFilter::All => true,
            EdgeFilter::Imports => edge_type == "import",
            EdgeFilter::Calls => edge_type == "call",
            EdgeFilter::Inherit => edge_type == "inherit",
        }
    }
}

/// A viewport rectangle: position + dimensions in world coordinates.
/// Replaces repeated `(x, y, w, h)` parameter tuples in layout functions.
#[derive(Debug, Clone, Copy)]
pub struct ViewportRect {
    /// Top-left X coordinate
    pub x: f64,
    /// Top-left Y coordinate
    pub y: f64,
    /// Width
    pub w: f64,
    /// Height
    pub h: f64,
}

impl ViewportRect {
    /// Create a new viewport rectangle.
    #[inline]
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
}

/// Recursive layout context — bundles the mutable output vectors and shared
/// read-only config that every recursive layout call needs. Reduces parameter
/// counts in blueprint.rs (place_children: 12→4, layout_dir: 11→4) and
/// treemap_layout.rs (layout_node: 10→4, layout_dir_children: 10→4).
pub struct LayoutCtx<'a> {
    /// Pre-computed per-file weights
    pub weights: &'a std::collections::HashMap<String, f64>,
    /// Output: positioned rectangles
    pub rects: &'a mut Vec<LayoutRectSlim>,
    /// Output: per-file anchor points
    pub anchors: &'a mut HashMap<String, Anchor>,
    /// Layout settings (min_rect, gutter, etc.)
    pub settings: &'a crate::core::settings::Settings,
}

// ─── Input enums ───────────────────────────────────────────────

/// What metric determines the visual size of each file block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeMode {
    /// Size by total line count
    Lines,
    /// Size by lines of executable logic
    Logic,
    /// Size by function/method count
    Funcs,
    /// Size by comment line count
    Comments,
    /// Size by blank line count
    Blanks,
    /// Size by live edit heat value
    Heat,
    /// Size by PMAT PageRank score (architecturally important files appear larger)
    PageRank,
    /// Size by degree centrality (highly connected files appear larger)
    Centrality,
    /// Size by clippy warning count (files with more warnings appear larger)
    ClippyCount,
    /// Equal size for all files
    Uniform,
}

/// Scaling transform applied to size values to compress extreme ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScaleMode {
    /// No transform (raw values)
    Linear,
    /// Square root scaling
    Sqrt,
    /// Logarithmic scaling
    Log,
    /// x^0.6 — best balance between linear and sqrt
    Smooth,
}

/// Spatial arrangement algorithm for file blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// Space-filling squarified treemap (Bruls et al. 2000)
    Treemap,
    /// Grid-based blueprint with fixed cell sizes
    Blueprint,
}

impl LayoutMode {
    /// Whether this mode uses the blueprint engine (viewport-independent sizing)
    pub fn is_blueprint(self) -> bool {
        matches!(self, LayoutMode::Blueprint)
    }
}

// ─── Output types ──────────────────────────────────────────────

/// Discriminant for layout rectangles: file, directory section, or root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RectKind {
    /// An individual source file block
    File,
    /// A directory section (container for files)
    Section,
    /// The root-level container
    Root,
}

/// Slim layout rectangle — geometry + identity only.
/// Renderers look up file metadata via `file_index.get(path)` from the Snapshot.
/// Keeps only what the layout engine computes; avoids duplicating FileNode data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutRectSlim {
    /// Relative file or directory path (key for metadata lookup)
    pub path: String,
    /// World-space X coordinate of top-left corner
    pub x: f64,
    /// World-space Y coordinate of top-left corner
    pub y: f64,
    /// Width in world units
    pub w: f64,
    /// Height in world units
    pub h: f64,
    /// Nesting depth (0 = top-level section)
    pub depth: u32,
    /// Whether this rect is a file, section, or root
    pub kind: RectKind,
    /// Path of the containing section (for edge routing)
    pub section_id: String,
    /// Blueprint grid coordinate (e.g. "A1") if using blueprint layout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid_coord: Option<String>,
    /// Header height in world units (for sections). Renderer uses this
    /// to draw the header strip at exactly the right size.
    #[serde(default)]
    pub header_h: f64,
}

/// Anchor point for connection routing and hit testing.
/// Each file block has one anchor at its center with block bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    /// File path this anchor belongs to
    pub file_path: String,
    /// Center X in world coordinates
    pub cx: f64,
    /// Center Y in world coordinates
    pub cy: f64,
    /// Containing section path
    pub section_id: String,
    /// Block top-left X (for edge routing exit points)
    pub bx: f64,
    /// Block top-left Y
    pub by: f64,
    /// Block width
    pub bw: f64,
    /// Block height
    pub bh: f64,
}

/// A single point in a routed edge path
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    /// X coordinate in world space
    pub x: f64,
    /// Y coordinate in world space
    pub y: f64,
}

/// Pre-routed polyline with color/alpha/style baked in.
/// Ready to draw: renderer just iterates points and paints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePath {
    /// Waypoints of the polyline in world coordinates
    pub pts: Vec<Point>,
    /// Red component of edge color (0-255)
    pub r: u8,
    /// Green component of edge color (0-255)
    pub g: u8,
    /// Blue component of edge color (0-255)
    pub b: u8,
    /// Opacity (0.0 = invisible, 1.0 = fully opaque)
    pub alpha: f64,
    /// Line width in screen pixels
    pub line_w: f64,
    /// Edge type label for filtering ("import", "call", "inherit")
    pub edge_type: String,
    /// Source file of this edge
    pub from_file: String,
    /// Target file of this edge
    pub to_file: String,
    /// Which border the edge exits from: 'l','r','t','b' (left/right/top/bottom).
    /// Set by the router. Renderer uses this for connector bar orientation.
    pub from_side: char,
}

// Badge rendering is done entirely in screen-space by renderer/badges.rs.
// No world-space Badge struct needed — removed dead code that was never
// read by the renderer (it iterates rd.rects + state.snapshot.entry_points).

/// Pre-computed per-file adjacency for O(1) spotlight lookups.
/// Keys = file path, Values = set of connected file paths per edge type.
/// Built once during layout aggregation instead of O(E) per frame.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeAdjacency {
    /// import edge neighbors
    pub import: HashMap<String, HashSet<String>>,
    /// call edge neighbors
    pub call: HashMap<String, HashSet<String>>,
    /// inherit edge neighbors
    pub inherit: HashMap<String, HashSet<String>>,
}

impl EdgeAdjacency {
    /// Get connected files for a given file and edge type filter.
    /// Uses a fixed-size array instead of Vec to avoid heap allocation per call.
    pub fn connected(&self, file: &str, edge_type: &str) -> HashSet<&str> {
        let mut set = HashSet::new();
        // Fixed-size array of up to 3 maps — avoids heap allocation from vec![]
        let (maps, count): ([&HashMap<String, HashSet<String>>; 3], usize) = match edge_type {
            "import" => ([&self.import, &self.import, &self.import], 1),
            "call" => ([&self.call, &self.call, &self.call], 1),
            "inherit" => ([&self.inherit, &self.inherit, &self.inherit], 1),
            _ => ([&self.import, &self.call, &self.inherit], 3), // "all"
        };
        for map in &maps[..count] {
            if let Some(neighbors) = map.get(file) {
                for n in neighbors {
                    set.insert(n.as_str());
                }
            }
        }
        set
    }
}

/// Complete pre-computed render data — flat, ready to draw.
/// Layout thread produces this; renderer consumes it without further computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderData {
    /// All file and section rectangles in world coordinates
    pub rects: Vec<LayoutRectSlim>,
    /// Per-file anchor points for edge routing and hit testing
    pub anchors: HashMap<String, Anchor>,
    /// Pre-routed edge polylines with color/alpha/style baked in
    pub edge_paths: Vec<EdgePath>,
    /// Total width of the layout content in world units
    pub content_width: f64,
    /// Total height of the layout content in world units
    pub content_height: f64,
    /// Margin reserved for edge routing outside file blocks
    pub route_margin: f64,
    /// Pre-computed adjacency index for O(1) spotlight lookups.
    /// Built once during edge aggregation, replacing O(E)-per-frame scanning.
    #[serde(default)]
    pub edge_adjacency: EdgeAdjacency,
}
