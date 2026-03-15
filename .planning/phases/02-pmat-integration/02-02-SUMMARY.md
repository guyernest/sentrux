---
phase: 02-pmat-integration
plan: 02
subsystem: renderer
tags: [pmat, tdg, colormode, serde, badges, treemap, color-interpolation]

requires:
  - phase: 02-pmat-integration
    plan: 01
    provides: "PmatReport, grade_to_display, grade_to_t from pmat_types.rs"

provides:
  - "Pruned ColorMode enum with 5 variants (Language, Heat, Git, TdgGrade, Monochrome) + serde(other)"
  - "tdg_grade_color() green-to-red gradient function in renderer/colors.rs"
  - "color_by_tdg_grade() dispatch using pmat_report lookup in renderer/rects.rs"
  - "pmat_report: Option<&PmatReport> field on RenderContext (ready for Plan 03)"
  - "pmat: Option<PmatReport> field on ScanReports (ready for Plan 03)"
  - "draw_tdg_badges() badge rendering with 28px threshold guard on treemap nodes"
  - "should_draw_tdg_badge() threshold helper (testable pure function)"
  - "TdgGrade as default ColorMode in AppState and UserPrefs"

affects:
  - 02-pmat-integration
  - 03-git-diff-overlay

tech-stack:
  added: []
  patterns:
    - "serde(other) fallback on last enum variant for backward-compatible deserialization"
    - "pmat_report: Option<&PmatReport> on RenderContext — None until populated by Plan 03"
    - "threshold guard helper: should_draw_tdg_badge(w, h) -> bool for testable UI logic"

key-files:
  created: []
  modified:
    - sentrux-core/src/layout/types.rs
    - sentrux-core/src/renderer/colors.rs
    - sentrux-core/src/renderer/rects.rs
    - sentrux-core/src/renderer/badges.rs
    - sentrux-core/src/renderer/mod.rs
    - sentrux-core/src/app/prefs.rs
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/app/update_loop.rs
    - sentrux-core/src/app/channels.rs
    - sentrux-core/src/app/scan_threads.rs

key-decisions:
  - "TdgGrade set as default ColorMode (replaces Monochrome/Language) — primary free mode per product direction"
  - "#[serde(other)] placed on Monochrome (last variant) so old prefs with churn/risk/age/execdepth/blastradius deserialize to Monochrome instead of erroring"
  - "All 5 remaining variants are non-Pro (is_pro() always returns false) — all survived the prune as essential modes"
  - "pmat_report on RenderContext initialized as None; Plan 03 will populate from AppState after scanner integration"
  - "pmat field added to ScanReports with None; Plan 03 scanner thread will set it via run_pmat_tdg()"

patterns-established:
  - "Grade color interpolation: t = grade_to_t(grade), r = 30 + (1-t)*225, g = 180*t, b = 40"
  - "Badge threshold guard: extract to should_draw_tdg_badge(w, h) -> bool for unit testability without egui painter"

requirements-completed: [PMAT-03, PMAT-04]

duration: 5min
completed: 2026-03-15
---

# Phase 02 Plan 02: ColorMode Prune + TDG Grade Coloring Summary

**Pruned ColorMode to 5 variants with serde(other) backward compat, TdgGrade green/red color gradient, and grade badge rendering with 28px threshold guard on treemap nodes**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-15T02:17:56Z
- **Completed:** 2026-03-15T02:22:51Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Rewrote ColorMode from 9 variants to 5 (Language, Heat, Git, TdgGrade, Monochrome) — old prefs with churn/risk/age deserialize to Monochrome via `#[serde(other)]`
- Implemented `tdg_grade_color()` with green(A+)-to-red(F) gradient using `grade_to_t()` from Plan 01, and wired up `color_by_tdg_grade()` dispatch using `pmat_report.by_path` lookup
- Added `draw_tdg_badges()` rendering letter grades (A+, B-, C, etc.) on treemap nodes with dark background pill for readability, skipping nodes below 28px

## Task Commits

Each task was committed atomically:

1. **Task 1: Prune ColorMode and add TdgGrade variant** - `a8cc679` (feat)
2. **Task 2: Add TDG grade badge rendering on treemap nodes** - `2a86453` (feat)

**Plan metadata:** (docs commit follows)

_Note: Both tasks followed TDD — tests written first, then implementation_

## Files Created/Modified

- `sentrux-core/src/layout/types.rs` - ColorMode pruned to 5 variants, TdgGrade added, serde(other) on Monochrome, 8 unit tests
- `sentrux-core/src/renderer/colors.rs` - Added `tdg_grade_color()` with 2 unit tests
- `sentrux-core/src/renderer/rects.rs` - Updated `file_color()` dispatch: removed Age/Churn/Risk/ExecDepth/BlastRadius arms, added TdgGrade arm with `color_by_tdg_grade()`
- `sentrux-core/src/renderer/badges.rs` - Added `draw_tdg_badges()`, `draw_tdg_grade_text()`, `should_draw_tdg_badge()` with 4 threshold unit tests
- `sentrux-core/src/renderer/mod.rs` - Added `pmat_report: Option<&PmatReport>` to RenderContext; called `draw_tdg_badges()` in `render_frame()`
- `sentrux-core/src/app/prefs.rs` - Default ColorMode changed to TdgGrade
- `sentrux-core/src/app/state.rs` - Default ColorMode changed to TdgGrade
- `sentrux-core/src/app/update_loop.rs` - Added `pmat_report: None` to RenderContext construction
- `sentrux-core/src/app/channels.rs` - Added `pmat: Option<PmatReport>` field to ScanReports
- `sentrux-core/src/app/scan_threads.rs` - Added `pmat: None` to ScanReports construction

## Decisions Made

- `TdgGrade` is the default color mode — product direction says it's the primary free mode, not Language
- `#[serde(other)]` on `Monochrome` (last variant) enables safe backward compat: old serialized prefs with removed variants silently become Monochrome
- All 5 remaining variants have `is_pro() = false` — they all survived the prune as essential, none warrant a Pro gate
- `pmat_report` initialized as `None` everywhere until Plan 03 wires up the scanner integration

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added pmat field to ScanReports**
- **Found during:** Task 1 (building after adding pmat_report to RenderContext)
- **Issue:** channels.rs test `scan_reports_has_pmat_field` was already written expecting `ScanReports.pmat` field (from Plan 01 test scaffolding), but the struct lacked the field — compile error
- **Fix:** Added `pub pmat: Option<PmatReport>` to ScanReports and `pmat: None` to the scan_threads construction
- **Files modified:** sentrux-core/src/app/channels.rs, sentrux-core/src/app/scan_threads.rs
- **Verification:** Build clean, test passes
- **Committed in:** a8cc679 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 - missing field completing pre-existing test contract)
**Impact on plan:** Necessary for correctness — the field was already tested and expected. No scope creep.

## Issues Encountered

None beyond the auto-fixed deviation above.

## Next Phase Readiness

- `RenderContext.pmat_report` is ready — Plan 03 just sets it from `AppState.pmat_report` in `update_loop.rs`
- `ScanReports.pmat` is ready — Plan 03 scanner thread calls `run_pmat_tdg()` and sets the field
- TdgGrade visual pipeline is complete: color gradient + badge rendering both work end-to-end
- Serde backward compat is in place: users upgrading from old prefs won't see crashes

## Self-Check: PASSED

- SUMMARY.md: FOUND
- sentrux-core/src/layout/types.rs: FOUND
- sentrux-core/src/renderer/badges.rs: FOUND
- Commit a8cc679 (Task 1): FOUND
- Commit 2a86453 (Task 2): FOUND

---
*Phase: 02-pmat-integration*
*Completed: 2026-03-15*
