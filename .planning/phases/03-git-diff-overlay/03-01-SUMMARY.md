---
phase: 03-git-diff-overlay
plan: 01
subsystem: visualization
tags: [git2, git-diff, color-mode, treemap, intensity-gradient, background-thread]

# Dependency graph
requires:
  - phase: 02.1-rust-deep-analysis
    provides: ColorMode enum with serde(other) Monochrome, RenderContext structure, colors.rs gradient pattern
provides:
  - FileDiffData with raw_intensity() geometric mean formula
  - GitDiffReport::from_walk aggregation from CommitRecords
  - FileAnalysisSnapshot and AnalysisSnapshot for metric delta storage
  - DiffWindow enum with PRESETS (7 entries), Default, Serialize/Deserialize
  - DiffWalkResult from walk_git_log_windowed
  - find_last_tag_epoch supporting both lightweight and annotated tags
  - ColorMode::GitDiff variant (serializes to "GitDiff") in ColorMode::ALL before Monochrome
  - git_diff_intensity_color(): blue-to-orange gradient
  - git_diff_new_file_color(): teal for new files
  - color_by_git_diff() dispatch arm in file_color()
  - spawn_git_diff_thread() in git_diff_adapter.rs
  - ScanMsg::GitDiffReady and GitDiffError variants
affects: [03-02-state-ui-plan, any future plan using GitDiff color mode or git walk windowed]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "DiffWindow enum separates three window modes (time/count/tag) with PRESETS const array"
    - "GitDiffReport::from_walk: CommitRecords aggregated to per-file intensity then normalized"
    - "Color gradients distinct by semantic meaning: blue-orange for diff (vs green-red for quality)"
    - "Background thread spawning with ScanMsg delivery: same pattern as CoverageReady"
    - "serde(rename = 'GitDiff') used to preserve exact case in user prefs despite rename_all = lowercase"

key-files:
  created:
    - sentrux-core/src/analysis/git_diff_adapter.rs
  modified:
    - sentrux-core/src/core/pmat_types.rs
    - sentrux-core/src/metrics/evo/git_walker.rs
    - sentrux-core/src/layout/types.rs
    - sentrux-core/src/renderer/colors.rs
    - sentrux-core/src/renderer/rects.rs
    - sentrux-core/src/renderer/mod.rs
    - sentrux-core/src/app/channels.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/scanning_tests.rs
    - sentrux-core/src/app/update_loop.rs

key-decisions:
  - "GitDiff serializes to 'GitDiff' (PascalCase) via serde(rename) to distinguish from lowercase rename_all convention — consistent with user-visible name"
  - "raw_intensity = sqrt(lines * commits): geometric mean prevents line-heavy single-commit spikes from dominating commit-frequency-heavy files"
  - "max_intensity defaults to 1.0 when no files changed: avoids division by zero and produces monochrome fallback"
  - "muted gray (50, 52, 55) for files not in git diff report (GDIT-04): distinguishes unchanged from no-data"
  - "CommitRecord/CommitFile made pub (was pub(crate)): git_diff_adapter.rs is in a sibling module"
  - "RenderContext.git_diff_report field added as None placeholder: Plan 02 wires actual AppState field"
  - "load_snapshot_at_boundary capped at 1000 commits: prevents infinite walk in repos with deep history"

patterns-established:
  - "DiffWindow::PRESETS: named time presets follow existing toolbar picker convention"
  - "On-demand git diff follows same spawn-thread → ScanMsg channel pattern as CoverageReady"

requirements-completed: [GDIT-01, GDIT-02, GDIT-03, GDIT-04]

# Metrics
duration: 7min
completed: 2026-03-15
---

# Phase 3 Plan 01: Git Diff Overlay Foundation Summary

**Git diff data pipeline from git2 revwalk to per-file blue-orange intensity colors: DiffWindow enum with 7 presets, GitDiffReport aggregation, teal new-file color, and background thread spawning via ScanMsg**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-15T06:55:06Z
- **Completed:** 2026-03-15T07:02:00Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Complete type foundation: FileDiffData, GitDiffReport, DiffWindow, AnalysisSnapshot all compile and pass tests
- ColorMode::GitDiff added to enum/ALL/label with correct serde and ordering (before Monochrome)
- Blue-to-orange gradient distinct from existing green-to-red quality gradients; teal for new files
- Git walker extended with walk_git_log_windowed and find_last_tag_epoch (handles both annotated and lightweight tags)
- Background thread pattern established for on-demand git diff analysis via ScanMsg channel

## Task Commits

Each task was committed atomically:

1. **Task 1: Types, ColorMode variant, and color functions** - `77f3770` (feat)
2. **Task 2: Git walker extension and diff adapter** - `2db5d13` (feat)

## Files Created/Modified

- `sentrux-core/src/analysis/git_diff_adapter.rs` - spawn_git_diff_thread, compute_git_diff_report, save/load analysis snapshots
- `sentrux-core/src/core/pmat_types.rs` - FileDiffData, GitDiffReport, FileAnalysisSnapshot, AnalysisSnapshot types
- `sentrux-core/src/metrics/evo/git_walker.rs` - DiffWindow enum, walk_git_log_windowed, find_last_tag_epoch, DiffWalkResult
- `sentrux-core/src/layout/types.rs` - ColorMode::GitDiff variant with serde rename, ALL array, label()
- `sentrux-core/src/renderer/colors.rs` - git_diff_intensity_color(), git_diff_new_file_color()
- `sentrux-core/src/renderer/rects.rs` - color_by_git_diff() dispatch arm in file_color()
- `sentrux-core/src/renderer/mod.rs` - RenderContext.git_diff_report field (None placeholder)
- `sentrux-core/src/app/channels.rs` - ScanMsg::GitDiffReady and GitDiffError variants
- `sentrux-core/src/app/scanning.rs` - Handle new ScanMsg variants (log and repaint)
- `sentrux-core/src/app/update_loop.rs` - git_diff_report: None in RenderContext constructor

## Decisions Made

- `#[serde(rename = "GitDiff")]` preserves PascalCase despite the enum's `rename_all = "lowercase"` — consistent with user-visible name and distinguishes from "gitdiff" typo
- raw_intensity = sqrt((lines_added + lines_removed) * commit_count): geometric mean combines volume and frequency without either dominating
- muted gray (50, 52, 55) for files not in the diff report: clearly distinguishes "unchanged in window" from "no data" (monochrome)
- RenderContext.git_diff_report field added as None placeholder to unblock compilation; Plan 02 wires the real AppState field

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated scanning.rs match arm for new ScanMsg variants**
- **Found during:** Task 2 (adding ScanMsg::GitDiffReady/GitDiffError)
- **Issue:** Non-exhaustive match in scanning.rs and scanning_tests.rs after adding new variants
- **Fix:** Added GitDiffReady handler (log + repaint) and GitDiffError handler (log + repaint) to poll_scan_messages(); added GitDiffReady/GitDiffError to scanning_tests.rs match arm
- **Files modified:** sentrux-core/src/app/scanning.rs, sentrux-core/src/app/scanning_tests.rs
- **Committed in:** 2db5d13 (Task 2 commit)

**2. [Rule 2 - Missing Critical] Added update_loop.rs RenderContext field initialization**
- **Found during:** Task 1 (adding git_diff_report to RenderContext)
- **Issue:** Struct initializer in update_loop.rs missing new field; compilation error
- **Fix:** Added `git_diff_report: None` to RenderContext construction in paint_render_frame()
- **Files modified:** sentrux-core/src/app/update_loop.rs
- **Committed in:** 77f3770 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (Rule 2 - missing critical for correct compilation)
**Impact on plan:** Both necessary for compilation. No scope creep.

## Issues Encountered

- Pre-existing test `color_mode_all_has_exactly_7_variants` needed update to 8 after adding GitDiff — straightforward, updated in-place
- Pre-existing 27 analysis::parser and analysis::graph oracle test failures are unrelated to this plan (removed language support from Phase 1)

## Next Phase Readiness

Plan 03-02 can now:
- Add `git_diff_report: Option<GitDiffReport>` to `AppState`
- Wire `git_diff_report: state.git_diff_report.as_ref()` in `RenderContext` construction
- Add toolbar window picker UI and trigger `spawn_git_diff_thread` on mode switch
- Store selected `DiffWindow` in prefs for session persistence

---
*Phase: 03-git-diff-overlay*
*Completed: 2026-03-15*
