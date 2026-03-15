//! User preferences that persist across app restarts.
//!
//! Saved/loaded via eframe's built-in storage (ron format). Captures the
//! user's layout mode, scale mode, size mode, color mode, theme, edge filter,
//! and panel visibility toggles. Automatically serialized on shutdown and
//! restored on launch so the UI remembers its last configuration.
//! Key type: `UserPrefs` (serializable subset of `AppState`).

use crate::layout::types::{LayoutMode, ScaleMode, SizeMode};
use crate::layout::types::ColorMode;
use crate::core::settings::Theme;
use crate::layout::types::EdgeFilter;
use crate::metrics::evo::git_walker::DiffWindow;
use serde::{Deserialize, Serialize};

const PREFS_KEY: &str = "sentrux_user_prefs";

fn default_diff_window() -> DiffWindow {
    DiffWindow::TimeSecs(86400)
}

fn default_custom_n() -> u32 {
    10
}

/// Serializable subset of AppState that persists across app restarts.
/// Stored in eframe's built-in ron-format storage.
#[derive(Serialize, Deserialize)]
pub struct UserPrefs {
    pub theme: Theme,
    pub color_mode: ColorMode,
    pub size_mode: SizeMode,
    pub scale_mode: ScaleMode,
    pub layout_mode: LayoutMode,
    pub edge_filter: EdgeFilter,
    pub show_all_edges: bool,
    pub activity_panel_open: bool,
    pub last_root_path: Option<String>,
    #[serde(default = "default_diff_window")]
    pub git_diff_window: DiffWindow,
    #[serde(default = "default_custom_n")]
    pub git_diff_custom_n: u32,
}

impl Default for UserPrefs {
    fn default() -> Self {
        Self {
            theme: Theme::Calm,
            color_mode: ColorMode::TdgGrade,
            size_mode: SizeMode::Lines,
            scale_mode: ScaleMode::Smooth,
            layout_mode: LayoutMode::Treemap,
            edge_filter: EdgeFilter::All,
            show_all_edges: false,
            activity_panel_open: false,
            last_root_path: None,
            git_diff_window: default_diff_window(),
            git_diff_custom_n: default_custom_n(),
        }
    }
}

#[cfg(test)]
mod prefs_tests {
    use super::*;
    use crate::metrics::evo::git_walker::DiffWindow;

    #[test]
    fn git_diff_window_roundtrips_via_serde() {
        let prefs = UserPrefs {
            git_diff_window: DiffWindow::CommitCount(5),
            git_diff_custom_n: 5,
            ..UserPrefs::default()
        };
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: UserPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.git_diff_window, DiffWindow::CommitCount(5));
        assert_eq!(restored.git_diff_custom_n, 5);
    }

    #[test]
    fn old_prefs_without_git_diff_window_deserialize_with_default() {
        // Simulate old prefs JSON that has no git_diff_window field
        // (uses lowercase for theme/size_mode/scale_mode/layout_mode/edge_filter per rename_all = "lowercase")
        let old_json = r#"{"theme":"calm","color_mode":"TdgGrade","size_mode":"lines","scale_mode":"smooth","layout_mode":"treemap","edge_filter":"all","show_all_edges":false,"activity_panel_open":false,"last_root_path":null}"#;
        let prefs: UserPrefs = serde_json::from_str(old_json).expect("should deserialize old prefs");
        assert_eq!(prefs.git_diff_window, DiffWindow::TimeSecs(86400), "default should be 1 day");
        assert_eq!(prefs.git_diff_custom_n, 10, "default custom_n should be 10");
    }

    #[test]
    fn git_diff_window_time_secs_roundtrips() {
        let prefs = UserPrefs {
            git_diff_window: DiffWindow::TimeSecs(604800),
            git_diff_custom_n: 10,
            ..UserPrefs::default()
        };
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: UserPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.git_diff_window, DiffWindow::TimeSecs(604800));
    }

    #[test]
    fn git_diff_window_since_last_tag_roundtrips() {
        let prefs = UserPrefs {
            git_diff_window: DiffWindow::SinceLastTag,
            git_diff_custom_n: 10,
            ..UserPrefs::default()
        };
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: UserPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.git_diff_window, DiffWindow::SinceLastTag);
    }
}

impl UserPrefs {
    /// Load from eframe storage, falling back to defaults.
    pub fn load(storage: &dyn eframe::Storage) -> Self {
        eframe::get_value(storage, PREFS_KEY).unwrap_or_default()
    }

    /// Save to eframe storage.
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PREFS_KEY, self);
    }

    /// Snapshot current app state into prefs.
    pub fn from_state(state: &crate::app::state::AppState) -> Self {
        Self {
            theme: state.theme,
            color_mode: state.color_mode,
            size_mode: state.size_mode,
            scale_mode: state.scale_mode,
            layout_mode: state.layout_mode,
            edge_filter: state.edge_filter,
            show_all_edges: state.show_all_edges,
            activity_panel_open: state.activity_panel_open,
            last_root_path: state.root_path.clone(),
            git_diff_window: state.git_diff_window,
            git_diff_custom_n: state.git_diff_custom_n,
        }
    }

    /// Apply saved prefs to app state.
    pub fn apply_to(&self, state: &mut crate::app::state::AppState) {
        state.theme = self.theme;
        state.theme_config = crate::core::settings::ThemeConfig::from_theme(self.theme);
        state.color_mode = self.color_mode;
        state.size_mode = self.size_mode;
        state.scale_mode = self.scale_mode;
        state.layout_mode = self.layout_mode;
        state.edge_filter = self.edge_filter;
        state.show_all_edges = self.show_all_edges;
        state.activity_panel_open = self.activity_panel_open;
        if self.last_root_path.is_some() {
            state.root_path = self.last_root_path.clone();
        }
        state.git_diff_window = self.git_diff_window;
        state.git_diff_custom_n = self.git_diff_custom_n;
    }
}
