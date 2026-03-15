---
phase: 04-gsd-phase-overlay
plan: 03
subsystem: ui
tags: [rust, egui, gsd, phase-navigator, treemap, toolbar, panels]

requires:
  - phase: 04-gsd-phase-overlay
    plan: 01
    provides: "PhaseStatus, PhaseInfo, GsdPhaseReport, gsd_phase_color(), DiffWindow::CommitRange"
  - phase: 04-gsd-phase-overlay
    plan: 02
    provides: "gsd_phase_running, gsd_phase_requested, selected_phase_idx, maybe_spawn_gsd_phase_thread(), draw_gsd_phase_section() stub"

provides:
  - "draw_gsd_phase_navigator() proportional timeline bar in draw_panels.rs: file-count-weighted segments, 40px minimum, click-to-GitDiff, hover tooltips, current-phase gold border, selected-phase gray border"
  - "draw_gsd_phase_legend() in draw_panels.rs: three-state swatches (Completed/InProgress/Planned/Not in any phase) plus coverage percentage stat"
  - "draw_gsd_phase_section() enhanced in pmat_panel.rs: Phase History collapsingheader, primary phase with status color, multi-phase history, amber actionable message for unassociated files"
  - "draw_gsd_phase_controls() in toolbar.rs: GSD separator, Refresh button, scanning spinner"
  - "GsdPhase status bar hover tooltip in status_bar.rs: phase name/number/status appended to file hover line"

affects: [visual-verification, phase-overlay-complete]

tech-stack:
  added: []
  patterns:
    - "Proportional bar via ui.allocate_exact_size + ui.painter() rects: more control than egui buttons for non-uniform segment widths"
    - "Post-iteration mutation: clone report before loop, collect mutations, apply after loop to satisfy borrow checker"
    - "Phase click navigation: sets git_diff_window = CommitRange and switches color_mode to GitDiff so user sees phase changes immediately"
    - "Status bar as hover tooltip surface: draw_left_info() appends colored phase label when GsdPhase mode active"

key-files:
  created: []
  modified:
    - sentrux-core/src/app/draw_panels.rs
    - sentrux-core/src/app/toolbar.rs
    - sentrux-core/src/app/panels/pmat_panel.rs
    - sentrux-core/src/app/status_bar.rs

key-decisions:
  - "Phase click navigation switches color_mode to GitDiff (not stays on GsdPhase): gives user immediate visual feedback of which files changed in that phase"
  - "Status bar used for treemap hover tooltip (GsdPhase info): avoids adding egui tooltip overlay to the canvas layer; status_bar.draw_left_info already has hovered_path and gsd_phase_report access"
  - "Proportional bar uses post-iteration mutation pattern: GsdPhaseReport cloned before draw loop so borrow checker allows state.selected_phase_idx update after loop"
  - "draw_gsd_phase_section checks all phases (not just by_file) for multi-phase history: by_file stores most-recent-wins index, but file may appear in earlier phases too"

patterns-established:
  - "Proportional bar segment sizing: natural_width = (count/total) * available_width, clamp to MIN_SEG_WIDTH, redistribute remaining to free segments"
  - "Amber actionable message pattern: files not in any phase get colored amber with review suggestion (not just gray/weak text)"

requirements-completed: [GSDP-02, GSDP-05]

duration: ~30min
completed: 2026-03-15
---

# Phase 4 Plan 3: GSD Phase Navigator, Legend, and Detail Panel Summary

**Proportional GSD phase timeline bar with file-count-weighted segments, click-to-GitDiff phase navigation, hover tooltips, color legend with coverage stat, and enhanced Phase History detail panel with multi-phase tracking and actionable unassociated-file messages**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-03-15
- **Completed:** 2026-03-15
- **Tasks:** 1 (+ visual verification checkpoint)
- **Files modified:** 4

## Accomplishments

- Proportional phase navigator bar renders in GsdPhase mode with segments sized by file count (40px minimum), gold border on current in-progress phase, gray border on selected phase
- Hover tooltips on each navigator segment show phase number, name, goal, status, and file count (GSDP-05)
- Clicking a phase segment sets GitDiff to that phase's commit range and switches color mode to GitDiff for immediate visual feedback
- GsdPhase color legend: Completed (green), In Progress (amber), Planned (steel-blue), Not in any phase (gray), plus "X% phase coverage" stat computed from snapshot file count
- Phase History section in file detail panel: primary phase with color-coded status, "Also modified in: Phase X, Y" for multi-phase files, amber actionable message for files not in any phase
- Status bar extended to show phase info (number, name, status) when hovering treemap nodes in GsdPhase mode
- Build clean: 276 tests pass, 27 pre-existing oracle failures unchanged

## Task Commits

1. **Task 1: Proportional phase navigator, color legend, detail panel, hover tooltips** - `a1ac93b` (feat)
2. **Refactor: simplify Phase 4 GSD overlay + fix phase click navigation** - `e962d9f` (refactor)

## Files Created/Modified

- `sentrux-core/src/app/draw_panels.rs` - draw_gsd_phase_navigator() proportional bar, draw_gsd_phase_legend() with coverage stat, draw_color_legend() GsdPhase branch
- `sentrux-core/src/app/toolbar.rs` - draw_gsd_phase_controls() Refresh button and spinner
- `sentrux-core/src/app/panels/pmat_panel.rs` - draw_gsd_phase_section() Phase History with multi-phase tracking and amber unassociated message
- `sentrux-core/src/app/status_bar.rs` - draw_left_info() extended with GsdPhase hover tooltip

## Decisions Made

- Phase click navigation switches `color_mode` to `ColorMode::GitDiff` immediately after setting `git_diff_window = CommitRange`: gives user a useful view (which files changed in that phase) rather than staying on GsdPhase with no visual change
- Status bar used as the treemap hover tooltip surface for GsdPhase: the `draw_left_info()` function already has access to `hovered_path` and `gsd_phase_report`, making it the natural place to append phase info without modifying the canvas rendering layer
- Proportional bar uses clone + post-iteration mutation: `report.clone()` before the draw loop, collect `new_selected`/`new_git_diff_window` during iteration, apply to `state` after loop to satisfy Rust borrow checker
- `draw_gsd_phase_section` iterates all `report.phases` to find multi-phase history, not just `by_file` index: `by_file` stores most-recent-phase-wins index, but a file may have been modified in earlier phases too

## Deviations from Plan

None — plan executed as specified. All four sub-tasks (navigator bar, detail panel, color legend, treemap hover tooltip) were implemented. The treemap hover tooltip was implemented via the status bar rather than canvas code, which was the natural integration point given existing architecture.

## Issues Encountered

None.

## Next Phase Readiness

- Complete GSD phase overlay feature ready for visual verification (checkpoint Task 2)
- User can switch to GsdPhase mode, see treemap colored by phase status, hover/click the navigator bar to explore phases, and click files to see phase history detail
- No blockers

---
*Phase: 04-gsd-phase-overlay*
*Completed: 2026-03-15*
