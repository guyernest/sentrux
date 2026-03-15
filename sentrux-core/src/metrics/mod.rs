//! Supplementary code metrics — evolution and test-gap analysis.
//!
//! PMAT (via subprocess) provides the primary code quality grades (TDG,
//! repo-score, coupling, complexity). This module provides only the metrics
//! that PMAT does not cover:
//!   - `evo`: bus factor, churn, temporal coupling via git log
//!   - `testgap`: untested high-risk file detection via import graph
//!
//! The old internal grading engine (grading.rs, stability.rs, arch/, dsm/,
//! rules/) has been deleted — replaced by `pmat quality-gate` and `pmat tdg`.

// ── Sub-modules ──
pub mod evo;        // evo/mod.rs + git_walker.rs
pub mod testgap;    // testgap analysis
pub mod types;      // shared metric types

