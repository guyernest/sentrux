---
phase: 05-improve-time-alignment
plan: 02
subsystem: timeline-navigator
tags: [timeline, navigator, egui, commits, milestones, tdd]
dependency_graph:
  requires:
    - CommitSummary, MilestoneInfo, TimelineSelection types (05-01)
    - GsdPhaseReport type (04-01)
  provides:
    - draw_timeline_navigator() replacing draw_gsd_phase_navigator()
    - AppState.commit_summaries, milestone_infos, timeline_selection fields
    - AppState.snapshot_write_requested, delta_requested flags
    - GsdPhaseReport.commits field (populated in gsd_phase_adapter)
    - collect_commit_summaries() in gsd_phase_adapter
    - RenderContext.delta_report field (wired for Plan 03)
  affects:
    - sentrux-core/src/app/draw_panels.rs
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/update_loop.rs
    - sentrux-core/src/core/pmat_types.rs
    - sentrux-core/src/analysis/gsd_phase_adapter.rs
    - sentrux-core/src/renderer/mod.rs
tech_stack:
  added: []
  patterns:
    - "equal_segment_rects() divides bar into equal-width sub-rects for O(1) layout"
    - "choose_tick_granularity_secs() selects tick interval by span order-of-magnitude"
    - "format_epoch_short() converts epoch to compact date string without chrono"
    - "collect_commit_summaries() walks git2 revwalk for per-commit metadata"
    - "Timeline selection uses Option<Option<T>> pattern: Some(None)=deselect, Some(Some(x))=select"
key_files:
  created: []
  modified:
    - sentrux-core/src/app/draw_panels.rs
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/update_loop.rs
    - sentrux-core/src/core/pmat_types.rs
    - sentrux-core/src/analysis/gsd_phase_adapter.rs
    - sentrux-core/src/renderer/mod.rs
decisions:
  - "Milestone bar hidden for milestones.len() <= 1 per CONTEXT.md: timeline is navigation, not color overlay"
  - "Timeline visible whenever gsd_phase_report is Some, regardless of color_mode"
  - "collect_commit_summaries() uses git2 revwalk in gsd_phase_adapter (background thread) not main thread handler"
  - "GsdPhaseReport carries commits Vec; scanning.rs handler extracts and moves to AppState"
  - "MilestoneInfo built in scanning.rs handler (deterministic from phases.len()), not in adapter"
  - "TimelineSelection derives PartialEq to enable deselect-on-reclick comparison"
  - "Commit row overflow: show N commits then '...' segment when segment width would go below 7px"
  - "choose_tick_granularity_secs thresholds: <=2h=600s, <=2d=14400s, <=60d=86400s, else=2592000s"
metrics:
  duration_minutes: 7
  completed_date: "2026-03-16"
  tasks_completed: 2
  files_modified: 7
---

# Phase 5 Plan 02: Timeline Navigator Widget Summary

**One-liner:** Replace proportional GSD phase bar with 3-tier timeline navigator (time ticks / phases / commits) using equal-width segments, git2 commit collection, and click-to-select interaction.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add timeline AppState fields and wire commit collection | e152294 | state.rs, scanning.rs, pmat_types.rs, gsd_phase_adapter.rs, renderer/mod.rs, update_loop.rs |
| 2 (RED) | Add failing tests for choose_tick_granularity_secs and equal_segment_rects | b672102 | draw_panels.rs |
| 2 (GREEN) | Replace draw_gsd_phase_navigator with draw_timeline_navigator | c49976a | draw_panels.rs, pmat_types.rs |

## What Was Built

### Task 1: AppState Fields and Commit Collection

Added to `AppState` (`state.rs`):
- `commit_summaries: Vec<CommitSummary>` — per-commit metadata for timeline bar
- `milestone_infos: Vec<MilestoneInfo>` — milestone groupings (single v1.0 milestone for now)
- `timeline_selection: Option<TimelineSelection>` — user's current timeline selection
- `snapshot_write_requested: bool` — triggers snapshot write on scan complete
- `delta_requested: bool` — triggers delta computation when selection changes

Added `commits: Vec<CommitSummary>` field to `GsdPhaseReport` in `pmat_types.rs`.

Added `collect_commit_summaries()` in `gsd_phase_adapter.rs`:
- Walks git2 revwalk (newest-to-oldest, up to 2000 commits)
- Computes `file_count` via `diff_tree_to_tree` (parent vs commit)
- Annotates each commit with `phase_idx` from scope regex (e.g., `feat(05-02):`)
- Sorts ascending by epoch for timeline display

Updated `GsdPhaseReady` handler in `scanning.rs`:
- Extracts commits from report → `state.commit_summaries`
- Builds single `v1.0` milestone → `state.milestone_infos`
- `clear_stale_state()` resets all timeline fields except `snapshot_write_running`

Added `delta_report: Option<&'a TimelineDeltaReport>` to `RenderContext` in `renderer/mod.rs`, wired from `state.timeline_delta_report` in `update_loop.rs`.

### Task 2: draw_timeline_navigator (TDD)

**RED:** 8 failing tests written for `choose_tick_granularity_secs` and `equal_segment_rects`.

**GREEN:** Full implementation of `draw_timeline_navigator`:

**Helper functions:**
- `choose_tick_granularity_secs(span_secs)` — returns tick interval: 60s/600s/14400s/86400s/2592000s based on span magnitude
- `equal_segment_rects(bar_rect, count)` — divides rect into N equal-width sub-rects
- `format_epoch_short(epoch, span)` — formats epoch as HH:MM / MMM DD / YYYY-MM without chrono

**draw_timeline_navigator layout (top to bottom):**
1. Time tick row (12px): tick lines and labels positioned at nearest-commit x for each tick epoch
2. Milestone row (16px): only rendered when `milestones.len() > 1`
3. Phase row (18px): equal-width, `gsd_phase_color()` fill, click=select/deselect, hover tooltip with goal (80 char)
4. Commit row (14px): equal-width, short_sha label at ≥40px width, overflow "..." segment when commits > 7px slots available

Removed `draw_gsd_phase_navigator()` (183 lines of proportional file-count segment code).

## Tests

| Test Module | Tests | Result |
|-------------|-------|--------|
| `draw_panels::timeline_tests` | 8 | Passed |

All 8 unit tests pass. 27 pre-existing oracle failures unchanged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed borrow-after-move on `resp.clicked()` in milestone row**
- **Found during:** Task 2 GREEN implementation
- **Issue:** `resp.on_hover_text()` consumed the `Response` value; calling `resp.clicked()` after was a compile error
- **Fix:** Captured `resp.clicked()` into `was_clicked` before calling `on_hover_text()`
- **Files modified:** draw_panels.rs
- **Commit:** c49976a

**2. [Rule 2 - Missing] Added PartialEq to TimelineSelection**
- **Found during:** Task 2 GREEN implementation
- **Issue:** Deselect-on-reclick comparison (`new_sel != current_selection`) requires PartialEq
- **Fix:** Added `PartialEq` to `#[derive]` on `TimelineSelection`
- **Files modified:** pmat_types.rs
- **Commit:** c49976a

**3. [Rule 2 - Missing] MilestoneInfo built in scanning.rs handler, not in adapter**
- **Found during:** Task 1
- **Issue:** Plan described building milestone_infos in adapter but it's simpler (and cleaner) to build in the handler since GsdPhaseReport doesn't need to carry milestone data
- **Fix:** Build `vec![MilestoneInfo { name: "v1.0"... }]` in `GsdPhaseReady` handler in scanning.rs
- **Files modified:** scanning.rs, gsd_phase_adapter.rs
- **Commit:** e152294

## Self-Check

### Files Modified
- `sentrux-core/src/app/draw_panels.rs` — exists
- `sentrux-core/src/app/state.rs` — exists
- `sentrux-core/src/app/scanning.rs` — exists
- `sentrux-core/src/app/update_loop.rs` — exists
- `sentrux-core/src/core/pmat_types.rs` — exists
- `sentrux-core/src/analysis/gsd_phase_adapter.rs` — exists
- `sentrux-core/src/renderer/mod.rs` — exists

### Commits
- `e152294` — Task 1 (AppState fields + commit collection)
- `b672102` — Task 2 RED (failing tests)
- `c49976a` — Task 2 GREEN (implementation)

## Self-Check: PASSED
