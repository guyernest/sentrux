---
phase: 06-ai-monitoring-ux
plan: 01
subsystem: renderer
tags: [risk-model, tdg-grade, pagerank, complexity-penalty, colors, rects]

# Dependency graph
requires:
  - phase: 02.1-rust-deep-analysis
    provides: "compute_raw_risk, risk_color, compute_max_risk_raw in colors.rs and rects.rs"
  - phase: 02-pmat-integration
    provides: "PmatReport.by_path, grade_to_t, TDG grade data on RenderContext"
provides:
  - "compute_raw_risk with 4th param complexity_penalty: f64"
  - "risk_color with complexity_penalty parameter"
  - "color_by_risk looks up TDG grade from ctx.pmat_report and passes penalty"
  - "compute_max_risk_raw with pmat param (conservative penalty=1.0 normalization)"
  - "A+ hub files (e.g. mod.rs) score near-zero in Risk color mode"
affects: [06-02-plan, 06-03-plan, future-risk-model-work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "complexity_penalty = 1.0 - grade_to_t(grade) as f64: maps A+ to 0.0 (suppressed) and F/unknown to 1.0 (full weight)"
    - "Conservative normalization: compute_max_risk_raw uses penalty=1.0 for all nodes so max is not under-estimated"
    - "TDD red-green: wrote failing tests calling new 4-arg signature first, then updated implementation"

key-files:
  created: []
  modified:
    - sentrux-core/src/renderer/colors.rs
    - sentrux-core/src/renderer/rects.rs
    - sentrux-core/src/app/scanning.rs

key-decisions:
  - "complexity_penalty = 1.0 - grade_to_t(grade) as f64: A+ yields 0.0 (near-zero risk), F/unknown yields 1.0 (full penalty)"
  - "compute_max_risk_raw uses conservative penalty=1.0 for all nodes: prevents under-estimating the maximum when A+ hub files dominate graph"
  - "pmat param added to compute_max_risk_raw signature for future use; currently only conservative 1.0 used (per RESEARCH.md pitfall)"

patterns-established:
  - "complexity_penalty derivation: always via grade_to_t lookup, never hardcoded"
  - "Unknown/missing grade always defaults to full penalty (1.0) — conservative risk model"

requirements-completed: [AIMON-01]

# Metrics
duration: 18min
completed: 2026-03-17
---

# Phase 6 Plan 01: TDG Grade complexity_penalty in Risk Formula Summary

**compute_raw_risk gains TDG grade complexity_penalty parameter: A+ hub files (mod.rs) score near-zero risk instead of false-alarming red due to high PageRank**

## Performance

- **Duration:** 18 min
- **Started:** 2026-03-17T02:38:00Z
- **Completed:** 2026-03-17T02:56:57Z
- **Tasks:** 1
- **Files modified:** 3

## Accomplishments
- Added `complexity_penalty: f64` as 4th parameter to `compute_raw_risk` with new formula `pr * penalty * uncovered * lint_factor`
- A+ grade (penalty=0.0) makes hub files near-zero risk; F/unknown grade (penalty=1.0) is identical to old formula
- `risk_color` gains `complexity_penalty` parameter and passes it to `compute_raw_risk`
- `color_by_risk` looks up TDG grade via `ctx.pmat_report.by_path` and computes `complexity_penalty`
- `compute_max_risk_raw` gains `pmat` parameter; uses conservative `penalty=1.0` for normalization (prevents under-estimating maximum)
- Both `compute_max_risk_raw` call sites in `scanning.rs` updated to pass `pmat_report`
- 3 new unit tests: `aplus_near_zero`, `f_grade_full_penalty`, `unknown_conservative`
- 347 existing tests continue to pass; 27 pre-existing oracle failures unchanged

## Task Commits

Each task was committed atomically:

1. **Task 1: Add complexity_penalty to compute_raw_risk and update all call sites** - `b55db72` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `sentrux-core/src/renderer/colors.rs` - compute_raw_risk gains 4th param; risk_color gains complexity_penalty; new test module compute_raw_risk_tests with 3 tests; existing risk_color tests updated to pass complexity_penalty
- `sentrux-core/src/renderer/rects.rs` - color_by_risk looks up TDG grade via ctx.pmat_report; compute_max_risk_raw gains pmat param, uses 1.0 conservative penalty
- `sentrux-core/src/app/scanning.rs` - Both compute_max_risk_raw call sites pass self.state.pmat_report.as_ref()

## Decisions Made
- `complexity_penalty = 1.0 - grade_to_t(grade) as f64`: A+ yields 0.0 (near-zero risk), F/unknown yields 1.0 (full penalty, same as old formula). This makes the new parameter a multiplier: penalty=1.0 is the identity transformation.
- `compute_max_risk_raw` uses conservative `penalty=1.0` for all nodes: prevents under-estimating the maximum when A+ hub files dominate the graph-metrics node list, which would cause other files to appear falsely red.
- `pmat` parameter added to `compute_max_risk_raw` signature for forward compatibility; currently unused internally (per RESEARCH.md pitfall note about conservative normalization).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Risk color mode now correctly reflects code complexity alongside connectivity and coverage
- A+ hub files (mod.rs, lib.rs, etc.) no longer false-alarm as red in Risk mode
- Ready for Phase 06-02 which builds on the updated risk model
- Pre-existing 27 parser oracle failures are unrelated to this work

## Self-Check: PASSED
- colors.rs: FOUND
- rects.rs: FOUND
- scanning.rs: FOUND
- SUMMARY.md: FOUND
- Commit b55db72: FOUND

---
*Phase: 06-ai-monitoring-ux*
*Completed: 2026-03-17*
