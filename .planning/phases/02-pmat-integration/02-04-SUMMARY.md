---
phase: 02-pmat-integration
plan: 04
subsystem: cleanup
tags: [pmat, metrics-deletion, subprocess, cli, dead-code-removal]

requires:
  - phase: 02-pmat-integration
    plan: 02
    provides: "Pruned ColorMode, ScanReports.pmat field, RenderContext.pmat_report"
  - phase: 02-pmat-integration
    plan: 03
    provides: "PMAT scan pipeline, AppState.pmat_report, pmat_panel.rs"

provides:
  - "metrics/arch/, metrics/dsm/, metrics/rules/, grading.rs, stability.rs deleted"
  - "metrics/evo/ and metrics/testgap/ preserved (PMAT does not cover these)"
  - "Old panels deleted: health_display, arch_display, dsm_panel, rules_display"
  - "ScanReports simplified: only evolution, test_gaps, pmat remain"
  - "AppState simplified: removed health_report, arch_report, rule_check_result, dsm_cache fields"
  - "sentrux check delegates to pmat quality-gate subprocess"
  - "sentrux gate delegates to pmat tdg --min-grade C subprocess"
  - "cargo build --workspace clean (no errors)"

affects:
  - 03-git-diff-overlay
  - 04-gsd-overlay

tech-stack:
  added: []
  patterns:
    - "CLI subprocess delegation: std::process::Command::new(pmat) with Stdio::inherit()"
    - "PMAT-not-found error pattern: Err(e) arm prints install instructions"

key-files:
  created: []
  modified:
    - sentrux-core/src/metrics/mod.rs
    - sentrux-core/src/metrics/types.rs
    - sentrux-core/src/app/channels.rs
    - sentrux-core/src/app/state.rs
    - sentrux-core/src/app/scan_threads.rs
    - sentrux-core/src/app/scanning.rs
    - sentrux-core/src/app/panels/mod.rs
    - sentrux-core/src/app/panels/metrics_panel.rs
    - sentrux-core/src/app/draw_panels.rs
    - sentrux-core/src/app/status_bar.rs
    - sentrux-core/src/app/toolbar.rs
    - sentrux-core/src/app/update_loop.rs
    - sentrux-core/src/renderer/mod.rs
    - sentrux-bin/src/main.rs
  deleted:
    - sentrux-core/src/metrics/arch/ (entire directory)
    - sentrux-core/src/metrics/dsm/ (entire directory)
    - sentrux-core/src/metrics/rules/ (entire directory)
    - sentrux-core/src/metrics/grading.rs
    - sentrux-core/src/metrics/stability.rs
    - sentrux-core/src/metrics/mod_tests.rs
    - sentrux-core/src/metrics/mod_tests2.rs
    - sentrux-core/src/metrics/test_helpers.rs
    - sentrux-core/src/app/panels/health_display.rs
    - sentrux-core/src/app/panels/arch_display.rs
    - sentrux-core/src/app/panels/dsm_panel.rs
    - sentrux-core/src/app/panels/rules_display.rs

key-decisions:
  - "metrics/types.rs retained as empty placeholder for future metric types rather than deleting the file"
  - "91 tests deleted along with deleted modules — all were testing now-removed functionality, not regressions"
  - "status_bar.rs now shows PMAT TDG average grade instead of internal health/arch grades"
  - "DSM panel toggle removed from toolbar (dsm_panel_open field removed from AppState)"
  - "update_check::record_scan now uses pmat repo_score grade (or '-' if unavailable) instead of HealthReport.grade"

requirements-completed: [PMAT-07, CLEN-04]

duration: 18min
completed: 2026-03-14
---

# Phase 02 Plan 04: Delete Replaced Metrics Engine and Rewire CLI Summary

**Deleted ~7000 lines of replaced internal grading engine (arch/dsm/rules/grading/stability), removed old panels, and rewired sentrux check/gate CLI to delegate to PMAT subprocess**

## Performance

- **Duration:** 18 min
- **Started:** 2026-03-14T11:15:57Z
- **Completed:** 2026-03-14T11:34:00Z
- **Tasks:** 2
- **Files modified:** 14 modified, 21 deleted

## Accomplishments

- Deleted the entire internal grading engine: `metrics/arch/`, `metrics/dsm/`, `metrics/rules/`, `grading.rs`, `stability.rs` — ~6900 lines of code replaced by PMAT
- Preserved `metrics/evo/` (bus factor, churn) and `metrics/testgap/` which PMAT does not cover
- Deleted four old panels (`health_display`, `arch_display`, `dsm_panel`, `rules_display`) and cleaned up all references
- Simplified `ScanReports` to only carry evolution, test_gaps, and pmat; simplified `AppState` to remove all deleted report fields
- Rewrote `run_check` to delegate to `pmat quality-gate --format json --fail-on-violation --path <root>`
- Rewrote `run_gate` to delegate to `pmat tdg --min-grade C --format json --path <root>`
- Both CLI commands handle PMAT-not-found gracefully with install instructions

## Task Commits

Each task was committed atomically:

1. **Task 1: Delete replaced metrics modules and old panels** - `3bae9d9` (feat)
2. **Task 2: Rewire sentrux check and gate to PMAT** - `f5abcbd` (feat)

## Files Created/Modified

**Deleted:**
- `sentrux-core/src/metrics/arch/` (mod.rs, distance.rs, graph.rs, tests.rs, tests2.rs)
- `sentrux-core/src/metrics/dsm/` (mod.rs, tests.rs)
- `sentrux-core/src/metrics/rules/` (mod.rs, checks.rs, tests.rs)
- `sentrux-core/src/metrics/grading.rs`, `stability.rs`, `mod_tests.rs`, `mod_tests2.rs`, `test_helpers.rs`
- `sentrux-core/src/app/panels/health_display.rs`, `arch_display.rs`, `dsm_panel.rs`, `rules_display.rs`

**Modified:**
- `sentrux-bin/src/main.rs` - Rewired run_check and run_gate to PMAT subprocess, removed all internal metrics imports
- `sentrux-core/src/metrics/mod.rs` - Simplified to only evo + testgap + types
- `sentrux-core/src/app/channels.rs` - ScanReports reduced to evolution/test_gaps/pmat
- `sentrux-core/src/app/state.rs` - Removed health_report, arch_report, rule_check_result, dsm_cache, dsm_panel_open fields
- `sentrux-core/src/app/scan_threads.rs` - Removed compute_health/arch/rules calls
- `sentrux-core/src/app/scanning.rs` - Removed arch degradation checks, simplified apply_scan_reports
- `sentrux-core/src/app/panels/mod.rs` - Removed arch_display, dsm_panel, health_display, rules_display modules
- `sentrux-core/src/app/panels/metrics_panel.rs` - Removed old panel calls, PMAT panel is now primary
- `sentrux-core/src/renderer/mod.rs` - Removed arch_report from RenderContext
- `sentrux-core/src/app/update_loop.rs` - Removed arch_report from RenderContext construction
- `sentrux-core/src/app/status_bar.rs` - Shows PMAT TDG average grade instead of internal grades
- `sentrux-core/src/app/toolbar.rs` - Removed DSM panel toggle button
- `sentrux-core/src/app/draw_panels.rs` - Removed DSM panel draw call

## Decisions Made

- `metrics/types.rs` retained as empty placeholder for future use rather than deleting the file entirely
- 91 test deletions are expected — they all tested the deleted grading/arch/dsm/rules/stability modules; 27 pre-existing test failures (oracle failures for removed languages) are unchanged
- `status_bar.rs` shows PMAT TDG average grade in the status bar as the primary grade signal
- `--save` flag in `sentrux gate` prints a warning message (not supported with PMAT gate) instead of erroring — preserves backward compat for CI scripts

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Build was clean after each phase. The `PmatRepoScore.overall_grade` field name was `grade` in the actual type (minor Rule 1 fix applied inline during Task 1, no separate commit needed).

## Next Phase Readiness

- Phase 02 is now complete — PMAT is fully integrated as the primary quality engine
- `sentrux check` and `sentrux gate` both delegate to PMAT with proper error handling
- No dead code from the deleted internal grading engine remains
- Phase 03 (git diff overlay) and Phase 04 (GSD overlay) can proceed cleanly

## Self-Check

- SUMMARY.md: FOUND
- Commit 3bae9d9 (Task 1): FOUND
- Commit f5abcbd (Task 2): FOUND
- sentrux-core/src/metrics/arch/: DELETED (confirmed)
- sentrux-core/src/app/panels/health_display.rs: DELETED (confirmed)
- sentrux-bin/src/main.rs contains "pmat": CONFIRMED
- cargo build --workspace: PASSED

## Self-Check: PASSED

---
*Phase: 02-pmat-integration*
*Completed: 2026-03-14*
