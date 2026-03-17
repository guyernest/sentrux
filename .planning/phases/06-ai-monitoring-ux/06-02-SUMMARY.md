---
phase: 06-ai-monitoring-ux
plan: 02
subsystem: renderer
tags: [git-diff, badges, treemap, rects, tdd]

# Dependency graph
requires:
  - phase: 03-git-diff-overlay
    provides: "GitDiffReport.by_file with FileDiffData (lines_added, lines_removed)"
  - phase: 06-ai-monitoring-ux
    provides: "RenderContext.git_diff_report field; ColorMode::GitDiff variant"
provides:
  - "draw_diff_badge(painter, rect, added, removed) — private fn in rects.rs"
  - "aggregate_dir_diff(by_file, dir_prefix) -> (u32, u32) — private fn in rects.rs"
  - "File rects in GitDiff mode show green +N and red -N at bottom-right"
  - "Directory rects in GitDiff mode show summed +/- counts across children"
affects: [06-03-plan]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "draw_diff_badge guards: rect.width() < 60.0 || rect.height() < 14.0 returns early; added==0 && removed==0 returns early"
    - "aggregate_dir_diff uses Box<dyn Iterator> to unify empty-prefix (all values) and prefix-filter (starts_with) paths"
    - "Removed text drawn rightmost at rect.right()-2, added text positioned to its left by text width + 3px gap"
    - "Badge uses Align2::RIGHT_BOTTOM at rect.bottom()-2 — distinct from draw_delta_arrow which uses RIGHT_TOP"

key-files:
  created: []
  modified:
    - sentrux-core/src/renderer/rects.rs

key-decisions:
  - "Badge minimum width 60px: '+1234 -567' is ~12 chars * 5px = 60px; narrower rects would overflow"
  - "Skip +0 -0 badges: zero-count files with no diff activity create visual noise with no information value"
  - "Removed drawn rightmost, added to its left: matches GitHub PR diff convention (del on right, add to left)"
  - "aggregate_dir_diff returns (u32, u32) not Option: zero aggregate is a valid and useful result for the caller"

patterns-established:
  - "diff badge wiring pattern: ColorMode::GitDiff gate -> git_diff_report guard -> badge call"
  - "directory prefix for aggregate: empty string for root/empty path, '{path}/' otherwise"

requirements-completed: [AIMON-02, AIMON-03]

# Metrics
duration: 3min
completed: 2026-03-17
---

# Phase 6 Plan 02: GitDiff +/- Line Count Badges Summary

**draw_diff_badge renders green +N and red -N text at bottom-right of file and directory rects when GitDiff color mode is active, using aggregate_dir_diff to sum child line counts for directories**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-17T02:59:39Z
- **Completed:** 2026-03-17T03:02:35Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Added `aggregate_dir_diff(by_file, dir_prefix) -> (u32, u32)` — sums lines_added/lines_removed across files matching prefix; empty prefix aggregates all (root query)
- Added `draw_diff_badge(painter, rect, added, removed)` — renders green +N at left, red -N at right of badge; guards on width<60px and both-zero cases
- Wired `draw_diff_badge` into `draw_file_rect` gated on `ColorMode::GitDiff` (AIMON-02)
- Wired `aggregate_dir_diff` + `draw_diff_badge` into `draw_section_rect` gated on `ColorMode::GitDiff` (AIMON-03)
- 4 new TDD tests in `diff_badge_tests` module; all pass; 352 pre-existing tests pass; 27 pre-existing oracle failures unchanged

## Task Commits

Each task was committed atomically:

1. **Task 1: Add draw_diff_badge, aggregate_dir_diff, and wire into draw_file_rect and draw_section_rect** - `11c2e6e` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `sentrux-core/src/renderer/rects.rs` — aggregate_dir_diff and draw_diff_badge functions added; both wired into draw_file_rect and draw_section_rect inside GitDiff mode gate; 4 unit tests added in diff_badge_tests module

## Decisions Made
- Badge minimum width 60px: "+1234 -567" is ~12 chars * 5px = 60px; narrower rects would overflow and the badge provides no value at that size
- Skip +0 -0 badges: files with zero added and zero removed within the diff window have no line-count signal to communicate; the color already conveys no-change via muted gray
- Removed text drawn rightmost, added to its left: matches GitHub's conventional PR diff layout (deletions on right side of the badge)
- `aggregate_dir_diff` returns `(u32, u32)` not `Option<(u32, u32)>`: zero aggregate is a meaningful valid result for root-level directories; caller guards on both-zero inside draw_diff_badge

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- GitDiff mode now shows both color intensity (how hot) and line counts (how much) in one view
- File rects show per-file +/- badges; directory rects show summed child counts
- Ready for Phase 06-03 visual checkpoint verification

## Self-Check: PASSED
- sentrux-core/src/renderer/rects.rs: FOUND
- SUMMARY.md: FOUND
- Commit 11c2e6e: FOUND

---
*Phase: 06-ai-monitoring-ux*
*Completed: 2026-03-17*
