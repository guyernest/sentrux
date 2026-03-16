---
phase: 05-improve-time-alignment
plan: 03
subsystem: timeline-wiring
tags: [timeline, snapshot, delta, git-diff, reset-button, tdd]
dependency_graph:
  requires:
    - write_analysis_snapshot, load_nearest_snapshot, compute_delta_report (05-01)
    - ScanMsg::SnapshotStored, DeltaReady, DeltaError (05-01)
    - AppState snapshot_write_requested, delta_requested, timeline_selection (05-01, 05-02)
    - draw_timeline_navigator, maybe_spawn_git_diff_thread pattern (05-02)
  provides:
    - maybe_spawn_snapshot_writer_thread() — spawns snapshot-writer background thread on scan complete
    - maybe_spawn_delta_thread() — spawns delta-compute background thread on selection change
    - draw_timeline_reset_button() — visible reset button when filter active
    - git_diff_window=CommitRange + git_diff_requested wired on selection change
    - snapshot_pipeline_should_start(), delta_pipeline_should_start() testable guard helpers
  affects:
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/draw_panels.rs
tech_stack:
  added: []
  patterns:
    - "flag-then-spawn: snapshot_write_requested set in apply_scan_reports, thread spawned next frame in draw_toolbar_panel"
    - "flag-then-spawn: delta_requested set in draw_timeline_navigator, thread spawned next frame in draw_toolbar_panel"
    - "load_nearest_snapshot + compute_delta_report chained in delta-compute background thread"
    - "Empty TimelineDeltaReport (no arrows) returned when no baseline snapshot exists"
    - "DiffWindow::CommitRange{from: sha, to: HEAD} built from selection SHA for git diff re-trigger"
    - "Testable guard helpers (snapshot_pipeline_should_start, delta_pipeline_should_start) for unit tests without thread spawning"
key_files:
  created: []
  modified:
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/draw_panels.rs
decisions:
  - "snapshot_write_requested set in apply_scan_reports (not handle_scan_complete) so it applies to both full scan and rescan paths"
  - "Empty TimelineDeltaReport (no arrows) when no baseline: correct per RESEARCH.md pitfall 3 — avoids spurious deltas"
  - "DiffWindow::CommitRange from=sha to=HEAD for Commit/Phase/Milestone selections; find earliest commit SHA in milestone"
  - "Reset button restores DiffWindow::default() and re-triggers git diff to update treemap to full history view"
  - "Guard helper functions extracted with #[allow(dead_code)] for testability without coupling to SentruxApp"
metrics:
  duration_minutes: 2
  completed_date: "2026-03-15"
  tasks_completed: 2
  files_modified: 2
---

# Phase 5 Plan 03: Snapshot Persistence Pipeline and Click-to-Filter Summary

**One-liner:** Wires snapshot write on scan completion and delta computation on timeline selection, completing the temporal filtering loop: click phase/commit -> delta thread + git diff thread -> treemap re-colors.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Wire snapshot write pipeline on scan completion | bbf28bb | scanning.rs, draw_panels.rs |
| 2 (TDD) | Wire delta computation pipeline, reset button, git diff re-trigger | be8c2ae | draw_panels.rs |

## What Was Built

### Task 1: Snapshot Write Pipeline (scanning.rs + draw_panels.rs)

**scanning.rs:**
- `apply_scan_reports`: Added `state.snapshot_write_requested = true` — triggers snapshot write on next frame after every scan completion
- `SnapshotStored` handler: Added logging `eprintln!("[snapshot] stored: {path}")` when path is non-empty

**draw_panels.rs:**
- `maybe_spawn_snapshot_writer_thread()`: Guard pattern — skips if running; clones pmat/coverage/clippy reports; spawns "snapshot-writer" thread; calls `write_analysis_snapshot`; sends `SnapshotStored(path)` on Ok, `SnapshotStored(String::new())` on Err
- `maybe_spawn_delta_thread()`: Guard pattern — skips if running; clears `delta_report` when no selection; otherwise loads nearest snapshot and computes delta; sends `DeltaReady(report)` (empty report when no baseline)
- `draw_toolbar_panel`: Wired both flag handlers after existing flag handlers

### Task 2: Delta Pipeline, Reset Button, Git Diff Re-trigger (draw_panels.rs, TDD)

**draw_timeline_navigator (mutations section):**
- On selection change: sets `delta_requested = true`
- On selection change: builds `DiffWindow::CommitRange { from: sha, to: "HEAD" }` from the selection's SHA (commit SHA, first commit in phase, or first commit in milestone's phases)
- Sets `git_diff_requested = true` to re-trigger the existing git diff thread
- On deselect (None): restores `DiffWindow::default()` and re-triggers git diff

**draw_timeline_reset_button():**
- Only rendered when `timeline_selection.is_some()`
- Amber-colored "x  Reset filter" button
- On click: clears `timeline_selection`, `timeline_delta_report`, `delta_requested`; restores default diff window; sets `git_diff_requested = true`

**Pipeline state helpers (testable):**
- `snapshot_pipeline_should_start()`: pure flag guard returning bool, no thread spawn
- `delta_pipeline_should_start()`: pure flag guard returning bool, handles no-selection case

## Tests

| Test | Result |
|------|--------|
| `test_snapshot_pipeline_state_transitions` | Passed |
| `test_snapshot_pipeline_not_requested` | Passed |
| `test_delta_pipeline_no_selection` | Passed |
| `test_delta_pipeline_with_selection` | Passed |
| `test_delta_pipeline_already_running` | Passed |

5 new tests, all passing. 304 pre-existing tests pass, 27 pre-existing oracle failures unchanged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] Added SnapshotStored logging**
- **Found during:** Task 1
- **Issue:** Plan specified `eprintln!("[snapshot] stored: {}", path)` but original handler had `_path` (silent)
- **Fix:** Updated handler to log when path is non-empty
- **Files modified:** scanning.rs
- **Commit:** bbf28bb

None beyond that — plan executed as written.

## Self-Check

### Files Modified
- `sentrux-core/src/app/scanning.rs` — exists
- `sentrux-core/src/app/draw_panels.rs` — exists

### Commits
- `bbf28bb` — Task 1 (snapshot write pipeline)
- `be8c2ae` — Task 2 (delta pipeline, reset button, git diff re-trigger)

## Self-Check: PASSED
