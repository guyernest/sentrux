---
phase: 03-git-diff-overlay
plan: 02
subsystem: state-management
tags: [git-diff, app-state, background-thread, prefs, pmat-panel, render-context]

# Dependency graph
requires:
  - phase: 03-01
    provides: GitDiffReport, DiffWindow, ScanMsg::GitDiffReady/GitDiffError, spawn_git_diff_thread, RenderContext.git_diff_report placeholder
provides:
  - AppState.git_diff_report, git_diff_running, git_diff_window, git_diff_requested, git_diff_custom_n
  - GitDiffReady/GitDiffError ScanMsg handlers storing report on AppState
  - maybe_spawn_git_diff_thread() background thread in draw_panels.rs
  - RenderContext.git_diff_report wired from state.git_diff_report.as_ref()
  - UserPrefs.git_diff_window and git_diff_custom_n with serde defaults for backward compat
  - draw_git_diff_section() in pmat_panel showing lines added/removed/commits in GitDiff mode
affects: [03-03-toolbar-ui (if planned), any plan reading state.git_diff_report or AppState git diff fields]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "git_diff_requested flag routes git diff spawn through draw_panels.rs — same pattern as coverage_requested"
    - "#[serde(default = 'fn')] on UserPrefs fields for backward-compatible deserialization of old prefs"
    - "New scan resets git_diff_report/running but preserves window/custom_n user selections"
    - "pmat_panel git diff section only shown when color_mode == ColorMode::GitDiff"

key-files:
  created: []
  modified:
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/draw_panels.rs
    - sentrux-core/src/app/update_loop.rs
    - sentrux-core/src/app/prefs.rs
    - sentrux-core/src/app/panels/pmat_panel.rs

key-decisions:
  - "git_diff_window and git_diff_custom_n NOT reset on new scan — these are user selections, not scan-scoped data"
  - "maybe_spawn_git_diff_thread wraps spawn_git_diff_thread in a named thread for identifiable panic traces"
  - "draw_git_diff_section only visible when color_mode == ColorMode::GitDiff — avoids noise in other modes"
  - "snapshot hint is sync disk check (.sentrux/snapshot.json exists) — no background thread needed for existence check"

# Metrics
duration: 4min
completed: 2026-03-15
---

# Phase 3 Plan 02: AppState Wiring and UserPrefs Summary

**Git diff pipeline wired through AppState, ScanMsg handlers, RenderContext, and UserPrefs: background thread spawning on request flag, session-persistent window selection, and file detail panel change metrics**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-15T14:25:24Z
- **Completed:** 2026-03-15T14:28:50Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- AppState carries all 5 git diff fields with correct defaults (report=None, running=false, window=1d, requested=false, custom_n=10)
- ScanMsg::GitDiffReady stores report on AppState and clears running flag; new scan resets report but preserves window selection
- maybe_spawn_git_diff_thread() in draw_panels.rs: guards on root/running/scanning, sets running=true, spawns named thread
- RenderContext.git_diff_report wired from state (was None placeholder in Plan 01)
- UserPrefs: 4 serde tests pass — round-trips TimeSecs, CommitCount, SinceLastTag; backward compat with old prefs JSON
- pmat_panel: Changes section shows lines added/removed/commits when file selected in GitDiff mode; snapshot hint guides users

## Task Commits

Each task was committed atomically:

1. **Task 1: AppState, ScanMsg handling, RenderContext, and background thread wiring** - `be61649` (feat)
2. **Task 2: UserPrefs persistence and detail panel metric deltas** - `797aeb9` (feat)

## Files Created/Modified

- `sentrux-core/src/app/state.rs` - 5 new git diff fields + DiffWindow/GitDiffReport imports + defaults in new()
- `sentrux-core/src/app/scanning.rs` - GitDiffReady/GitDiffError handlers updated; reset in clear_stale_state and apply_scan_reports
- `sentrux-core/src/app/draw_panels.rs` - git_diff_requested handler + maybe_spawn_git_diff_thread()
- `sentrux-core/src/app/update_loop.rs` - git_diff_report: state.git_diff_report.as_ref() (was None)
- `sentrux-core/src/app/prefs.rs` - git_diff_window/git_diff_custom_n fields + serde defaults + from_state/apply_to wiring + 4 tests
- `sentrux-core/src/app/panels/pmat_panel.rs` - draw_git_diff_section() for GitDiff mode file detail

## Decisions Made

- git_diff_window and git_diff_custom_n not reset on new scan — these are user selections that survive project changes
- maybe_spawn_git_diff_thread wraps spawn_git_diff_thread in a std::thread::Builder named "git-diff" for identifiable panic traces
- draw_git_diff_section only shown when color_mode == ColorMode::GitDiff to avoid noise in other modes
- snapshot hint uses sync disk check (exists()) — lightweight, no background thread needed for existence query alone

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- Pre-existing 27 oracle test failures (removed language support from Phase 1) unchanged — not caused by this plan

## Self-Check

All committed files verified present and all commits verified in git log below.

## Next Phase Readiness

Phase 03-03 can now:
- Add toolbar UI to set git_diff_requested=true and cycle through DiffWindow options
- Read state.git_diff_running to show spinner while background thread runs
- Read state.git_diff_window to display current window selection
- The full pipeline is live: toolbar sets flag → draw_panels spawns thread → ScanMsg delivers report → RenderContext colors files

---
*Phase: 03-git-diff-overlay*
*Completed: 2026-03-15*
