---
phase: 04-gsd-phase-overlay
plan: 02
subsystem: app-pipeline
tags: [rust, gsd, pipeline, background-thread, toolbar, panels]

requires:
  - phase: 04-gsd-phase-overlay
    plan: 01
    provides: "GsdPhaseReport, parse_gsd_phases(), ScanMsg::GsdPhaseReady/GsdPhaseError, ColorMode::GsdPhase, gsd_phase_color(), color_by_gsd_phase()"

provides:
  - "gsd_phase_running, gsd_phase_requested, selected_phase_idx fields on AppState"
  - "maybe_spawn_gsd_phase_thread() background thread spawner in draw_panels.rs"
  - "GsdPhaseReady/Error handlers clear gsd_phase_running flag in scanning.rs"
  - "GsdPhase auto-trigger in toolbar draw_visual_group()"
  - "draw_gsd_phase_section() in pmat_panel.rs for file detail"
  - "clear_stale_state() resets gsd_phase_report/running on new scan"

affects: [04-03-plan, phase-navigator, toolbar, panels]

tech-stack:
  added: []
  patterns:
    - "maybe_spawn_gsd_phase_thread follows coverage/git-diff pattern: guard root/running/scanning, set running flag, spawn named thread, send ScanMsg"
    - "Auto-trigger fires on mode switch: ColorMode::GsdPhase && prev != GsdPhase && report.is_none && !running && !scanning"

key-files:
  created: []
  modified:
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/draw_panels.rs
    - sentrux-core/src/app/toolbar.rs
    - sentrux-core/src/app/panels/pmat_panel.rs

key-decisions:
  - "gsd_phase_running and gsd_phase_requested added to AppState following the exact git_diff_running/git_diff_requested pattern; selected_phase_idx initialized to None for Plan 03 navigator"
  - "draw_gsd_phase_section uses inline use crate::core::pmat_types::PhaseStatus for narrow import scope"
  - "clear_stale_state resets gsd_phase_report/running but NOT selected_phase_idx (user selection persists across directory switches per CONTEXT.md)"

requirements-completed: [GSDP-01, GSDP-03, GSDP-04]

duration: 3min
completed: 2026-03-15
---

# Phase 4 Plan 2: GSD Phase Pipeline Wiring Summary

**Full GSD phase data pipeline wired from disk parsing through background thread, ScanMsg delivery, AppState storage, and RenderContext to treemap coloring — with auto-trigger on mode switch and file detail panel**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-15
- **Completed:** 2026-03-15
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `gsd_phase_running`, `gsd_phase_requested`, `selected_phase_idx` added to AppState with correct initialization
- `GsdPhaseReady`/`GsdPhaseError` scan handlers updated to clear `gsd_phase_running` flag (Plan 01 had them but missing the flag clear)
- `clear_stale_state()` now resets `gsd_phase_report`/`gsd_phase_running` on new scan (does NOT reset `selected_phase_idx`)
- `maybe_spawn_gsd_phase_thread()` added to draw_panels.rs following the established coverage/git-diff pattern
- `gsd_phase_requested` flag handler added in `draw_toolbar_panel()` after the git_diff handler
- Auto-trigger in `draw_visual_group()`: fires when switching to GsdPhase with no report, not running, not scanning
- `draw_gsd_phase_section()` in pmat_panel.rs shows phase number, name, and color-coded status (Completed=green, InProgress=amber, Planned=steel-blue)
- UserPrefs already persists GsdPhase via `color_mode: ColorMode` field (verified, no changes needed)
- 276 tests pass; 27 pre-existing oracle failures unchanged

## Task Commits

1. **Task 1: AppState fields, ScanMsg handlers, background thread** - `38e4211` (feat)
2. **Task 2: GsdPhase auto-trigger and file detail panel section** - `b3d196e` (feat)

## Pipeline Verification

Full data flow confirmed:
```
toolbar sets gsd_phase_requested = true
  -> draw_panels.rs catches flag, calls maybe_spawn_gsd_phase_thread()
  -> thread calls parse_gsd_phases(&root)
  -> ScanMsg::GsdPhaseReady(report) sent
  -> scanning.rs stores state.gsd_phase_report = Some(report), clears gsd_phase_running
  -> update_loop.rs RenderContext.gsd_phase_report = state.gsd_phase_report.as_ref()
  -> rects.rs file_color() dispatches to color_by_gsd_phase() (Plan 01)
```

## Files Modified

- `sentrux-core/src/app/state.rs` - gsd_phase_running, gsd_phase_requested, selected_phase_idx fields
- `sentrux-core/src/app/scanning.rs` - clear gsd_phase_running in handlers, reset in clear_stale_state
- `sentrux-core/src/app/draw_panels.rs` - maybe_spawn_gsd_phase_thread(), request handler
- `sentrux-core/src/app/toolbar.rs` - GsdPhase auto-trigger in draw_visual_group()
- `sentrux-core/src/app/panels/pmat_panel.rs` - draw_gsd_phase_section() with phase info display

## Decisions Made

- Used inline `use` import for `PhaseStatus` inside `draw_gsd_phase_section()` to keep import scope narrow (function only uses it there)
- `selected_phase_idx` left as `None` in clear_stale_state — matches decision from CONTEXT.md that user phase navigation should persist across directory switches
- No new UserPrefs fields needed — `color_mode: ColorMode` already serializes GsdPhase via Plan 01's `#[serde(rename = "GsdPhase")]`

## Deviations from Plan

None — plan executed exactly as written. Plan 01 had already partially wired some items (gsd_phase_report field on AppState, partial scan handlers, RenderContext field) but was missing the running/requested flags and the background thread. This plan completed the remaining wiring without conflicts.

## Issues Encountered

None.

## Next Phase Readiness

- Full data pipeline complete: GSD phase coloring is live end-to-end
- `selected_phase_idx` field ready for Plan 03's phase navigator panel
- No blockers for Plan 03

---
*Phase: 04-gsd-phase-overlay*
*Completed: 2026-03-15*
