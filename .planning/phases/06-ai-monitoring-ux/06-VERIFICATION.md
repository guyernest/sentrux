---
phase: 06-ai-monitoring-ux
verified: 2026-03-16T00:00:00Z
status: passed
score: 9/9 must-haves verified
re_verification: false
human_verification:
  - test: "Open sentrux on this project (which has an InProgress phase)"
    expected: "App opens in GitDiff color mode showing the current phase's commit range immediately"
    why_human: "Requires running the GUI application; auto-diff fires on GsdPhaseReady which is a background scan event"
  - test: "In GitDiff mode, zoom into individual files in the treemap"
    expected: "File rects wider than 60px show green '+N' and red '-N' text at bottom-right; directory rects show summed counts"
    why_human: "Badge rendering via egui::Painter cannot be verified by grep; requires visual inspection"
  - test: "Click a phase in the timeline navigator, then open a new project folder"
    expected: "The auto-diff does NOT override your existing timeline selection"
    why_human: "User interaction flow — requires running the app and making a manual timeline selection before loading GSD phase data"
---

# Phase 6: AI Monitoring UX Verification Report

**Phase Goal:** Optimize the treemap for monitoring AI code assistants in real-time — phase-aware change visibility (default to current phase's changes instead of fading heat), git diff +/- line counts on file nodes, and a smarter risk model that weights centrality by complexity and coverage gaps instead of raw PageRank

**Verified:** 2026-03-16
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                      | Status     | Evidence                                                                                                  |
|----|-------------------------------------------------------------------------------------------|------------|-----------------------------------------------------------------------------------------------------------|
| 1  | A file with TDG grade A+ and high PageRank is NOT red in Risk mode                        | VERIFIED   | `compute_raw_risk(0.9, 50.0, 5, 0.0) < 0.001` confirmed by test; penalty=0.0 forces raw=0.0 → green      |
| 2  | A file with TDG grade F and high PageRank IS red in Risk mode (same as before)            | VERIFIED   | `compute_raw_risk(0.9, 50.0, 5, 1.0)` equals old 3-arg formula confirmed by `f_grade_full_penalty` test   |
| 3  | `compute_raw_risk` returns near-zero for A+ grade even with high PageRank                 | VERIFIED   | Test `compute_raw_risk_aplus_near_zero` passes; formula: `pr * 0.0 * uncovered * lint_factor = 0`        |
| 4  | `compute_raw_risk` returns same value as before for unknown grade (full penalty)          | VERIFIED   | Test `compute_raw_risk_unknown_conservative` passes; unknown → penalty=1.0 (conservative)                 |
| 5  | In GitDiff mode, a file rect shows '+N -N' badge at bottom-right with green/red text      | VERIFIED   | `draw_diff_badge` wired in `draw_file_rect` under `ColorMode::GitDiff` gate; 4 TDD tests pass            |
| 6  | A rect narrower than 60px shows no badge (too small to render without overflow)           | VERIFIED   | Guard `rect.width() < 60.0 \|\| rect.height() < 14.0` in `draw_diff_badge` at rects.rs:253              |
| 7  | A directory rect shows the summed +/- across all child files                              | VERIFIED   | `aggregate_dir_diff` wired in `draw_section_rect` under `ColorMode::GitDiff` gate; prefix filter tested  |
| 8  | On app open with InProgress GSD phase, color_mode switches to GitDiff automatically       | VERIFIED   | `try_apply_auto_diff` wired in `GsdPhaseReady` handler; `auto_diff_triggers_on_in_progress_phase` passes |
| 9  | When the user has a timeline_selection, the auto-switch does NOT fire                     | VERIFIED   | Guard `state.timeline_selection.is_some()` returns early; `auto_diff_blocked_by_timeline_selection` passes|

**Score:** 9/9 truths verified

---

### Required Artifacts

| Artifact                                      | Expected                                          | Status      | Details                                                                              |
|-----------------------------------------------|---------------------------------------------------|-------------|--------------------------------------------------------------------------------------|
| `sentrux-core/src/renderer/colors.rs`         | `compute_raw_risk` with `complexity_penalty: f64` | VERIFIED    | 4th param added at line 127; formula `pr * penalty * uncovered * lint_factor`        |
| `sentrux-core/src/renderer/colors.rs`         | `risk_color` with `complexity_penalty: f64`       | VERIFIED    | 5th param added at line 149; passes through to `compute_raw_risk`                    |
| `sentrux-core/src/renderer/rects.rs`          | `draw_diff_badge` function                        | VERIFIED    | Private fn at line 252; guards on width<60 and both-zero                             |
| `sentrux-core/src/renderer/rects.rs`          | `aggregate_dir_diff` function                     | VERIFIED    | Private fn at line 228; returns `(u32, u32)` summed from prefix-filtered children    |
| `sentrux-core/src/renderer/rects.rs`          | `color_by_risk` with TDG grade lookup             | VERIFIED    | Looks up via `ctx.pmat_report.by_path` at line 705; computes penalty                 |
| `sentrux-core/src/renderer/rects.rs`          | `compute_max_risk_raw` with `pmat` parameter      | VERIFIED    | 4th param `_pmat` added at line 726; uses conservative penalty=1.0 for normalization |
| `sentrux-core/src/app/state.rs`               | `auto_diff_active: bool` on `AppState`            | VERIFIED    | Field at line 214; initialized false in `new()` at line 346                          |
| `sentrux-core/src/app/scanning.rs`            | `try_apply_auto_diff` function                    | VERIFIED    | Private fn at line 756; guards on timeline_selection, uses rposition()               |

---

### Key Link Verification

| From                                      | To                                                              | Via                                                   | Status   | Details                                                                                     |
|-------------------------------------------|-----------------------------------------------------------------|-------------------------------------------------------|----------|---------------------------------------------------------------------------------------------|
| `color_by_risk` in rects.rs              | `colors::risk_color` (→ `compute_raw_risk`)                    | passes `complexity_penalty` derived from TDG grade    | WIRED    | rects.rs:709-711; pattern `compute_raw_risk.*complexity_penalty` confirmed                  |
| `compute_max_risk_raw` in rects.rs       | `colors::compute_raw_risk`                                      | passes conservative `penalty=1.0` for normalization   | WIRED    | conservative 1.0 hardcoded; `_pmat` param accepted for future use                          |
| `draw_file_rect` → `draw_diff_badge`     | called after `draw_delta_arrow` when `ColorMode::GitDiff`       | rects.rs:398-402                                      | WIRED    | Gate on `ctx.color_mode == ColorMode::GitDiff`, then `git_diff_report` guard               |
| `draw_section_rect` → `aggregate_dir_diff` + `draw_diff_badge` | summed dir counts in GitDiff mode        | rects.rs:161-169                                      | WIRED    | Both gates confirmed; dir_prefix derived from `r.path`                                     |
| `GsdPhaseReady` handler → `try_apply_auto_diff` | called BEFORE `state.gsd_phase_report = Some(report)`   | scanning.rs:102                                       | WIRED    | Pattern `auto_diff_active = true` confirmed in implementation; borrow-safe order            |
| `try_apply_auto_diff` → state fields     | sets `color_mode`, `git_diff_window`, `git_diff_requested`, `auto_diff_active` | conditional on InProgress + no timeline selection | WIRED    | All 4 state fields set inside `if let Some((from_sha, _to_sha)) = in_progress` block        |
| Both `compute_max_risk_raw` call sites in scanning.rs | pass `self.state.pmat_report.as_ref()`            | scanning.rs:65 and scanning.rs:173                    | WIRED    | Both call sites updated; verified by grep                                                   |

---

### Requirements Coverage

| Requirement | Source Plan | Description                                                                               | Status    | Evidence                                                                                          |
|-------------|------------|-------------------------------------------------------------------------------------------|-----------|---------------------------------------------------------------------------------------------------|
| AIMON-01    | 06-01      | Risk color mode multiplies by TDG complexity_penalty so A+ hub files no longer false-alarm | SATISFIED | `compute_raw_risk` 4th param; `color_by_risk` grade lookup; 3 unit tests pass                    |
| AIMON-02    | 06-02      | File rects in GitDiff mode show green "+N" and red "-N" badges at bottom-right           | SATISFIED | `draw_diff_badge` wired in `draw_file_rect`; 60px guard; zero-count guard                        |
| AIMON-03    | 06-02      | Directory rects in GitDiff mode show summed +/- line counts across all children           | SATISFIED | `aggregate_dir_diff` wired in `draw_section_rect`; 4 TDD tests pass including prefix filter      |
| AIMON-04    | 06-03      | On app open with InProgress GSD phase, color_mode auto-switches to GitDiff               | SATISFIED | `try_apply_auto_diff` wired in `GsdPhaseReady`; 5 unit tests pass                                |
| AIMON-05    | 06-03      | Auto-diff does not override a user's existing timeline_selection                          | SATISFIED | Guard `timeline_selection.is_some()` returns early; `auto_diff_blocked_by_timeline_selection` passes |

No orphaned requirements — all 5 AIMON IDs mapped in REQUIREMENTS.md are claimed by a plan and verified.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `sentrux-core/src/renderer/rects.rs` | 512 | `pub(crate) fn compute_delta_net_score` — dead_code warning | Info | Unused function from prior work; pre-existing, unrelated to phase 6 |

No blockers. No stubs. No placeholder implementations.

---

### Human Verification Required

#### 1. Auto-diff Activation on Launch

**Test:** Open sentrux pointed at a GSD project that has an InProgress phase with a commit_range.
**Expected:** Treemap immediately renders in GitDiff color mode (blue-to-orange gradient) with the InProgress phase's commit range; no manual action required.
**Why human:** Requires running the GUI application; the `GsdPhaseReady` handler fires from a background scan thread and triggers `try_apply_auto_diff` — this flow cannot be exercised without launching the app.

#### 2. GitDiff Badge Rendering

**Test:** In GitDiff color mode, zoom into files that have non-zero lines_added or lines_removed.
**Expected:** File rects wider than 60px display green "+N" text and red "-N" text at the bottom-right corner; directory rects display summed counts for all children; rects narrower than 60px show no badge.
**Why human:** Badge rendering via `egui::Painter::text()` cannot be verified by static analysis — requires visual inspection of the running treemap.

#### 3. Timeline Selection Protection

**Test:** Click a phase in the timeline navigator (setting a `timeline_selection`), then reload the project or trigger a rescan that fires `GsdPhaseReady`.
**Expected:** Your manual timeline selection is preserved; the app does NOT auto-switch to GitDiff mode or override your selected commit range.
**Why human:** Requires user interaction to set a timeline_selection followed by a rescan event — an integration-level flow not exercised by unit tests.

---

### Test Results Summary

| Test Suite                          | Passing | Failing | Notes                                     |
|-------------------------------------|---------|---------|-------------------------------------------|
| `compute_raw_risk_*`                | 3/3     | 0       | New tests from plan 06-01                 |
| `risk_color_*`                      | 3/3     | 0       | Existing tests; updated for new signature |
| `diff_badge_tests::*`               | 4/4     | 0       | New tests from plan 06-02                 |
| `auto_diff_scan_tests::*`           | 5/5     | 0       | New tests from plan 06-03                 |
| `new_state_*` (state.rs)            | 2/2     | 0       | Regression check; auto_diff_active=false  |
| Full suite (`cargo test -p sentrux-core`) | 356/385 | 27 (pre-existing) | All 27 failures are oracle/parser tests from prior phases; zero phase-6 regressions |

---

### Commit Verification

All three plan commits are present in git history:

| Commit  | Plan  | Description                                              |
|---------|-------|----------------------------------------------------------|
| b55db72 | 06-01 | feat(06-01): add TDG grade complexity_penalty to risk formula |
| 11c2e6e | 06-02 | feat(06-02): draw_diff_badge and aggregate_dir_diff for GitDiff mode |
| 0210495 | 06-03 | feat(06-03): auto-switch to GitDiff mode on InProgress GSD phase |

---

_Verified: 2026-03-16_
_Verifier: Claude (gsd-verifier)_
