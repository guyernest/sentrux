//! Metrics/display panel sub-module — UI panels for PMAT analysis,
//! evolution, test gaps, and activity display.
//!
//! All files in this module were extracted from `src/app/` to improve
//! module cohesion. They form a natural cluster: `metrics_panel.rs`
//! orchestrates the others, and most use `ui_helpers::dim_grade_color`.

pub(crate) mod activity_panel;
pub(crate) mod evolution_display;
pub(crate) mod metrics_panel;
pub(crate) mod pmat_panel;
pub(crate) mod testgap_display;
pub(crate) mod ui_helpers;
