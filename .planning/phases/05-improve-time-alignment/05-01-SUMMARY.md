---
phase: 05-improve-time-alignment
plan: 01
subsystem: data-foundation
tags: [types, snapshot, delta, timeline, persistence]
dependency_graph:
  requires: []
  provides:
    - CommitSummary, MilestoneInfo, TimelineSelection, TimelineSelectionKind types
    - TimelineDeltaReport, FileDeltaEntry types
    - grade_to_rank(), grade_delta() functions
    - ScanMsg::SnapshotStored, DeltaReady, DeltaError variants
    - write_analysis_snapshot, load_nearest_snapshot, compute_delta_report, prune_snapshots
  affects:
    - sentrux-core/src/core/pmat_types.rs
    - sentrux-core/src/app/channels.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/analysis/snapshot_writer.rs
tech_stack:
  added: [tempfile (dev-dependency)]
  patterns:
    - "Snapshot files named {epoch}.json for O(1) epoch-sorted lookup"
    - "grade_to_rank() integer mapping enables grade_delta() signed arithmetic"
    - "compute_delta_report skips files only in baseline or only in current (no comparison basis)"
key_files:
  created:
    - sentrux-core/src/analysis/snapshot_writer.rs
  modified:
    - sentrux-core/src/core/pmat_types.rs
    - sentrux-core/src/app/channels.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/scanning_tests.rs
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/analysis/mod.rs
    - sentrux-core/Cargo.toml
decisions:
  - "grade_to_rank unknown returns -1; grade_delta returns 0 if either side is unknown to avoid spurious deltas"
  - "compute_delta_report only produces entries for files present in both baseline and current (not new or deleted files)"
  - "prune_snapshots sorts ascending by epoch filename integer, deletes oldest first"
  - "snapshot_write_running, delta_running, timeline_delta_report added to AppState following gsd_phase_running/requested pattern"
metrics:
  duration_minutes: 4
  completed_date: "2026-03-16"
  tasks_completed: 2
  files_modified: 8
---

# Phase 5 Plan 01: Timeline Data Types and Snapshot Writer Summary

**One-liner:** Timeline types (CommitSummary, TimelineDeltaReport, grade_to_rank/grade_delta) and snapshot persistence (write/load/prune/delta) establish the data foundation for Phase 5 timeline navigation.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Define timeline types and ScanMsg variants | a8e28aa | pmat_types.rs, channels.rs, scanning.rs, scanning_tests.rs, state.rs |
| 2 | Implement snapshot writer, loader, delta computation, and pruning | 27338e5 | snapshot_writer.rs (new), mod.rs, Cargo.toml |

## What Was Built

### Task 1: Timeline Types (pmat_types.rs + channels.rs)

Added to `sentrux-core/src/core/pmat_types.rs`:
- `CommitSummary` — per-commit metadata (sha, short_sha, message, author, epoch, file_count, phase_idx)
- `MilestoneInfo` — named milestone covering phase indices (name, phase_indices)
- `TimelineSelectionKind` enum — Milestone / Phase / Commit
- `TimelineSelection` — user's current timeline selection (kind, index, epoch_start)
- `FileDeltaEntry` — per-file metric deltas (tdg_grade_delta, coverage_pct_delta, clippy_count_delta)
- `TimelineDeltaReport` — delta comparison result (by_file HashMap, baseline_epoch)
- `grade_to_rank(grade: &str) -> i32` — maps APLus=10..F=0, unknown=-1
- `grade_delta(old, new) -> i32` — returns 0 when either grade is unknown

Added to `sentrux-core/src/app/channels.rs`:
- `ScanMsg::SnapshotStored(String)` — path of written snapshot file
- `ScanMsg::DeltaReady(TimelineDeltaReport)` — computed delta report
- `ScanMsg::DeltaError(String)` — delta computation failure

Updated exhaustive matches in `scanning.rs` and `scanning_tests.rs`.

Added to `AppState` (state.rs): `snapshot_write_running`, `timeline_delta_report`, `delta_running`.

### Task 2: Snapshot Writer (analysis/snapshot_writer.rs)

Four public functions implementing snapshot persistence:
1. `write_analysis_snapshot` — builds AnalysisSnapshot from PMAT/coverage/clippy, writes to `.sentrux/snapshots/{epoch}.json`, calls prune
2. `load_nearest_snapshot` — finds largest epoch <= target_epoch, deserializes and returns
3. `compute_delta_report` — computes per-file deltas for files in both baseline and current; skips new/deleted files
4. `prune_snapshots` — deletes oldest snapshots (ascending epoch sort) when count > max_count

## Tests

| Test Module | Tests | Result |
|-------------|-------|--------|
| `pmat_types::timeline_tests` | 7 | Passed |
| `analysis::snapshot_writer::tests` | 9 | Passed |

Total: 16 new tests, all passing.

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check

### Files Created
- `sentrux-core/src/analysis/snapshot_writer.rs` — exists

### Commits
- `a8e28aa` — exists (Task 1)
- `27338e5` — exists (Task 2)

## Self-Check: PASSED
