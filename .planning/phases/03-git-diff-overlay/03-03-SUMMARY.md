---
phase: 03-git-diff-overlay
plan: 03
subsystem: ui
tags: [git-diff, toolbar, color-legend, egui, window-selector, auto-trigger]

# Dependency graph
requires:
  - phase: 03-01
    provides: DiffWindow::PRESETS, git_diff_intensity_color(), git_diff_new_file_color(), ColorMode::GitDiff
  - phase: 03-02
    provides: AppState git diff fields (git_diff_report, git_diff_running, git_diff_window, git_diff_requested, git_diff_custom_n), maybe_spawn_git_diff_thread()
provides:
  - draw_git_diff_controls(): toolbar window preset buttons (15m/1h/1d/1w/tag/1c/5c) and custom N input
  - draw_color_legend(): mode-dispatched color legend strip below toolbar (GitDiff/TdgGrade/Coverage/Risk)
  - Auto-trigger behavior: switching TO GitDiff mode with no report sets git_diff_requested=true
  - NO_DATA_GRAY constant extracted for consistent muted-gray reuse across files
affects: [any future plan adding new ColorMode variants (must add legend branch), phase 04+ UI work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "draw_git_diff_controls() only renders when state.color_mode == ColorMode::GitDiff — follows single-mode-guard pattern from pmat_panel"
    - "draw_color_legend() dispatches by color_mode: GitDiff/TdgGrade/Coverage/Risk get unique legends; other modes return early"
    - "Color legend swatches use painter.rect_filled() with small/weak RichText labels for unobtrusive rendering"
    - "Auto-trigger guards: !state.git_diff_running && state.git_diff_report.is_none() && !state.scanning before setting git_diff_requested"

key-files:
  created: []
  modified:
    - sentrux-core/src/app/toolbar.rs
    - sentrux-core/src/app/draw_panels.rs
    - sentrux-core/src/analysis/git_diff_adapter.rs
    - sentrux-core/src/app/panels/pmat_panel.rs
    - sentrux-core/src/app/prefs.rs
    - sentrux-core/src/metrics/evo/git_walker.rs
    - sentrux-core/src/renderer/colors.rs
    - sentrux-core/src/renderer/rects.rs

key-decisions:
  - "Color legend placed below toolbar as second row in draw_toolbar_panel TopBottomPanel — avoids modifying canvas/panel layout code"
  - "NO_DATA_GRAY constant extracted (was duplicated magic literal in 3 files) — centralizes the (50,52,55) muted-gray value"
  - "Auto-trigger guards !state.scanning to prevent triggering a git diff walk while a full scan is already running"
  - "load_snapshot_at_boundary uses continue (not ?) on OID errors — invalid OIDs skip gracefully instead of aborting the walk"
  - "SinceLastTag with no tags returns empty report instead of unbounded walk — prevents freeze on repos with no tags"
  - "spawn_git_diff_thread eliminated: compute_git_diff_report inlined into thread closure — removes double-indirection"

patterns-established:
  - "Toolbar controls gated on color_mode: prevents UI clutter for non-active modes"
  - "Legend dispatch by color_mode: each mode owns its legend rendering; unknown modes return early silently"

requirements-completed: [OVRL-01, OVRL-02]

# Metrics
duration: ~50min
completed: 2026-03-15
---

# Phase 3 Plan 03: Toolbar UI and Color Legend Summary

**Toolbar window preset buttons (15m/1h/1d/1w/tag/1c/5c), mode-dispatched color legend strip, and auto-trigger on GitDiff mode switch — completing the user-facing interaction for the git diff overlay**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-03-15T07:25:00Z
- **Completed:** 2026-03-15T08:20:23Z
- **Tasks:** 2 (Task 1 auto, Task 2 human-verify checkpoint — approved)
- **Files modified:** 8

## Accomplishments

- Toolbar shows GitDiff window selector row (15m/1h/1d/1w/tag/1c/5c presets + custom N input) only when ColorMode::GitDiff is active
- Color legend strip rendered below toolbar for GitDiff (muted/gradient/new-file swatches), TdgGrade (grade badges), Coverage (coverage gradient), and Risk (risk gradient) modes
- Auto-trigger: switching TO GitDiff with no existing report fires git_diff_requested=true immediately (guarded against concurrent scan and running thread)
- Post-checkpoint refactor: eliminated double-thread spawn, fixed SinceLastTag unbounded walk, extracted NO_DATA_GRAY constant, added !state.scanning auto-trigger guard

## Task Commits

Each task was committed atomically:

1. **Task 1: Toolbar window selector, auto-trigger, and color legend** - `a3dd567` (feat)
2. **Task 2: Visual verification — APPROVED** - checkpoint (no commit)
3. **Post-checkpoint simplify refactor** - `77ec1fe` (refactor)

## Files Created/Modified

- `sentrux-core/src/app/toolbar.rs` - draw_git_diff_controls() with preset buttons and custom N input; auto-trigger on GitDiff mode switch
- `sentrux-core/src/app/draw_panels.rs` - draw_color_legend() dispatching per ColorMode; wired as second row in toolbar panel
- `sentrux-core/src/analysis/git_diff_adapter.rs` - Inlined compute_git_diff_report into thread (removed double-spawn), fixed OID error handling with continue
- `sentrux-core/src/metrics/evo/git_walker.rs` - Fixed SinceLastTag with no tags to return empty; pre-computed prefix_sep before hot loop
- `sentrux-core/src/renderer/colors.rs` - Extracted NO_DATA_GRAY constant (was magic literal in 3 places)
- `sentrux-core/src/renderer/rects.rs` - Updated to use NO_DATA_GRAY constant
- `sentrux-core/src/app/panels/pmat_panel.rs` - Removed per-frame p.exists() stat call
- `sentrux-core/src/app/prefs.rs` - Replaced duplicate default fns with #[serde(default)]

## Decisions Made

- Color legend placed as second row in the existing `draw_toolbar_panel` TopBottomPanel — avoids touching canvas/panel layout code and keeps toolbar self-contained
- NO_DATA_GRAY constant extracted to colors.rs: (50, 52, 55) muted gray was duplicated in rects.rs, colors.rs, and draw_panels.rs
- Auto-trigger guard includes `!state.scanning` to prevent starting a git diff walk during a full project scan (race condition)
- SinceLastTag with no tags now returns an empty GitDiffReport instead of falling through to an unbounded walk — prevents freeze on fresh repos
- spawn_git_diff_thread() removed; compute_git_diff_report inlined into the spawned thread closure — eliminates unnecessary indirection

## Deviations from Plan

None — plan executed exactly as written. The post-checkpoint refactor (77ec1fe) was additional cleanup committed after user visual approval, not a deviation from planned tasks.

## Issues Encountered

- None during Task 1 implementation. Refactor commit after verification fixed several correctness issues discovered during review (unbounded SinceLastTag walk, per-frame disk stat call, magic literal duplication).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Phase 03-git-diff-overlay is complete. The full git diff overlay pipeline is live:
- User switches to GitDiff → auto-triggers background walk → report stored on AppState → RenderContext colors treemap files by intensity
- Toolbar presets and custom N input let users explore different time windows
- Color legend explains the visualization for all active modes
- Preferences persist window selection across restarts

Phase 04 (GSD overlay) can now proceed. No blockers from Phase 03.

---
*Phase: 03-git-diff-overlay*
*Completed: 2026-03-15*
