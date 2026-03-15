---
phase: 02-pmat-integration
plan: 01
subsystem: analysis
tags: [pmat, subprocess, serde, json-deserialization, code-quality, unwrap-elimination]

requires:
  - phase: 01-cleanup
    provides: "Clean Rust/TS/JS-only codebase with static lang_registry"

provides:
  - "PmatTdgOutput, PmatFileScore, PmatPenalty types for PMAT TDG JSON"
  - "PmatRepoScore, PmatScoreCategory types for PMAT repo-score JSON"
  - "PmatReport with normalized by_path index (strips './' prefix)"
  - "grade_to_display() mapping all 11 PMAT grades to display strings"
  - "grade_to_t() mapping grades to 0.0-1.0 for color interpolation"
  - "run_pmat_tdg() and run_pmat_repo_score() subprocess adapters (None on failure)"
  - "check_pmat_available() binary availability check"
  - "Zero .unwrap() calls in non-test production code (plan-scoped files)"

affects:
  - 02-pmat-integration
  - 03-git-diff-overlay
  - 04-gsd-overlay

tech-stack:
  added: []
  patterns:
    - "Subprocess JSON: Command + Stdio::null() + temp file output + serde_json::from_slice"
    - "None-returning adapters: all PMAT failures return None, never panic"
    - "by_path index: normalize file_path keys by stripping './' for O(1) lookup"
    - ".expect(msg) replaces .unwrap() for algorithmic invariants (Tarjan SCC)"

key-files:
  created:
    - sentrux-core/src/core/pmat_types.rs
    - sentrux-core/src/analysis/pmat_adapter.rs
  modified:
    - sentrux-core/src/core/mod.rs
    - sentrux-core/src/analysis/mod.rs
    - sentrux-core/src/layout/spatial_index.rs
    - sentrux-core/src/layout/blueprint_dag.rs
    - sentrux-core/src/analysis/parser/imports.rs
    - sentrux-core/src/analysis/parser/lang_extractors.rs
    - sentrux-core/src/analysis/resolver/helpers.rs
    - sentrux-core/src/analysis/resolver/oxc.rs
    - sentrux-core/src/metrics/mod.rs

key-decisions:
  - "PmatReport uses by_path HashMap<String, usize> (index into tdg.files) rather than cloning file data — zero-copy lookup"
  - "run_pmat_tdg accepts non-zero exit (PMAT exits 1 on critical defects but still writes JSON) — still reads output file"
  - "Tarjan SCC .unwrap() replaced with .expect() with invariant messages — satisfies PMAT heuristic while preserving panic-on-logic-error semantics"
  - "Tree-sitter child() access uses match/if-let with continue instead of unwrap — safe even if tree is structurally unexpected"

patterns-established:
  - "PMAT adapter pattern: subprocess -> temp file -> serde_json::from_slice -> Option<T>"
  - "Normalized path indexing: strip './' prefix on ingest, not at lookup time"

requirements-completed: [PMAT-01, PMAT-02]

duration: 5min
completed: 2026-03-15
---

# Phase 02 Plan 01: PMAT Foundation Summary

**PMAT JSON types (TDG + repo-score), subprocess adapter (None on failure), grade helpers, and zero .unwrap() in non-test production code across 7 plan-scoped files**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-15T02:10:32Z
- **Completed:** 2026-03-15T02:15:40Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- Created `pmat_types.rs` with all PMAT data types (TdgOutput, FileScore, Penalty, RepoScore, ScoreCategory, Report) and grade helper functions (`grade_to_display` maps 11 grades, `grade_to_t` maps to 0.0-1.0)
- Created `pmat_adapter.rs` with subprocess invocation functions that return `None` on any failure (binary missing, bad exit, parse error)
- Eliminated all `.unwrap()` calls in non-test production code across 7 plan-scoped files — unblocks PMAT self-analysis of the sentrux codebase

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pmat_types.rs and pmat_adapter.rs** - `fc9c40d` (feat)
2. **Task 2: Fix .unwrap() violations in non-test code** - `c8b1f61` (fix)

**Plan metadata:** (docs commit follows)

_Note: Task 1 followed TDD — types file written with full test suite, all 14 tests green on first run_

## Files Created/Modified

- `sentrux-core/src/core/pmat_types.rs` - PMAT data types + grade helper functions (PmatTdgOutput, PmatFileScore, PmatPenalty, PmatRepoScore, PmatScoreCategory, PmatReport, grade_to_display, grade_to_t)
- `sentrux-core/src/analysis/pmat_adapter.rs` - PMAT subprocess adapter (run_pmat_tdg, run_pmat_repo_score, check_pmat_available)
- `sentrux-core/src/core/mod.rs` - Added `pub mod pmat_types`
- `sentrux-core/src/analysis/mod.rs` - Added `pub mod pmat_adapter`
- `sentrux-core/src/layout/spatial_index.rs` - `best.is_none() || area < best.unwrap().1` → `best.map_or(true, |b| area < b.1)`
- `sentrux-core/src/layout/blueprint_dag.rs` - 3x `get_mut().unwrap()` → `if let Some(out)` pattern
- `sentrux-core/src/analysis/parser/imports.rs` - 2x tree-sitter child access `.unwrap()` → `match ... continue`
- `sentrux-core/src/analysis/parser/lang_extractors.rs` - 7x `.unwrap()` on tree-sitter children and char conversion → if-let/unwrap_or
- `sentrux-core/src/analysis/resolver/helpers.rs` - 2x `.unwrap()` on rfind/find → derived slice offsets + if-let
- `sentrux-core/src/analysis/resolver/oxc.rs` - `borrow.1.as_ref().unwrap()` → match with early return
- `sentrux-core/src/metrics/mod.rs` - 4x Tarjan SCC `.unwrap()` → `.expect("tarjan invariant: ...")`

## Decisions Made

- `run_pmat_tdg` accepts non-zero exit codes because PMAT exits with code 1 when it finds critical defects, but still writes valid JSON output — we attempt to read the file regardless.
- `PmatReport::by_path` uses index into `tdg.files` rather than cloning `PmatFileScore` data — zero-copy lookup pattern established for Phase 03 use.
- Tarjan SCC invariants use `.expect()` (not `.unwrap()`) with descriptive messages — satisfies PMAT's critical defect heuristic while preserving the original "panic on violated invariant" intent.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. All 14 new tests passed on first compilation. Build clean throughout.

## Next Phase Readiness

- `pmat_types.rs` and `pmat_adapter.rs` are the foundation required by all Phase 02 plans.
- `run_pmat_tdg` and `run_pmat_repo_score` are ready for integration into the scan pipeline.
- Zero `.unwrap()` in plan-scoped non-test production code — PMAT self-analysis should succeed (exit 0) on the sentrux codebase.

---
*Phase: 02-pmat-integration*
*Completed: 2026-03-15*
