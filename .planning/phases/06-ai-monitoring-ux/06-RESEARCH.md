# Phase 06: AI Monitoring UX - Research

**Researched:** 2026-03-16
**Domain:** Rust/egui treemap rendering — risk model extension, git diff badge rendering, phase-aware auto-diff
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Phase-aware change visibility**
- Auto-switch to GitDiff on open: when the app opens on a GSD project with an in-progress phase, auto-set `DiffWindow::CommitRange` to that phase's commit range and switch to GitDiff color mode.
- Keep heat overlay alongside: heat ripples continue for real-time feedback. GitDiff coloring shows phase-scoped changes. Different layers, different purposes.
- Auto-refresh on new commits: when the watcher detects a file change during an active phase, auto-re-trigger the git diff to include the new commit.
- Any phase via timeline click: auto-diff defaults to in-progress phase. User can click any phase in the timeline to view its changes (existing Phase 5 behavior — just making the default smarter).

**Git diff +/- counts on nodes**
- Bottom-right badge: small "+42 -7" text at bottom-right of file rects. Only shows when GitDiff mode is active.
- GitHub-style green/red coloring: "+42" in green, "-7" in red.
- Directory aggregation: directory rects show summed +/- counts across all children.

**Smarter risk model**
- Formula: `centrality × complexity_penalty × coverage_gap × lint_factor`
  - `centrality` = PageRank (existing)
  - `complexity_penalty` = TDG grade converted to 0.0–1.0 multiplier (A+=0.0, A=0.1, B+=0.2...F=1.0)
  - `coverage_gap` = 1.0 − coverage% (existing)
  - `lint_factor` = ln(clippy+1) (existing)
- Simple hub files (mod.rs with grade A+) get near-zero complexity_penalty → negligible risk.
- No-coverage default: files without coverage data get 50% assumption (kept).
- No user-tunable weights: fixed formula for v1.

**Change persistence across sessions**
- Phase commit range as session boundary: on reopen, detect in-progress phase, use its first commit as baseline. No extra prefs state — GSD phase boundary IS the natural session boundary.
- Works across restarts: phase commit ranges parsed from git history.

### Claude's Discretion
- Minimum rect size for showing +/- badges (below which they are hidden)
- Whether to show +0/-0 for files in the diff range but with no line changes
- How to handle the auto-switch when multiple phases are in-progress (pick the highest numbered one)
- Animation/transition when auto-switching color mode on open

### Deferred Ideas (OUT OF SCOPE)
- User-tunable risk weights in settings panel
- Animated playback of changes within a phase
- Notification/alert when risk score increases significantly during AI work
</user_constraints>

---

## Summary

Phase 6 has three independent capabilities that each touch a well-defined layer of the existing codebase. No new Rust dependencies or architectural patterns are required — every change builds on patterns already established in phases 3, 4, and 5.

**Feature 1 — Phase-aware auto-diff:** After the GSD phase parse completes (`ScanMsg::GsdPhaseReady`), `apply_scan_reports` adds logic to detect the highest-numbered `InProgress` phase, extract its `commit_range`, set `DiffWindow::CommitRange`, and trigger a git diff. The `AppState` gains an `auto_diff_active: bool` flag to track whether the auto-switch happened, so it can be distinguished from a user-initiated timeline click. On watcher-triggered rescans, the same flag drives a re-trigger of the git diff. The key constraint: the auto-switch must NOT overwrite a user-chosen `pre_timeline_color_mode` save, and must NOT fire if the user has already made a timeline selection.

**Feature 2 — +/- badge rendering:** `FileDiffData` already contains `lines_added` and `lines_removed` per file. A new `draw_diff_badge()` function in `rects.rs` renders bottom-right text in the same pattern as `draw_delta_arrow()`. Directory aggregation follows the existing `aggregate_dir_delta()` pattern — sum `lines_added`/`lines_removed` across children. The badge only renders when `ctx.color_mode == ColorMode::GitDiff` and `ctx.git_diff_report.is_some()`.

**Feature 3 — Smarter risk model:** `compute_raw_risk()` in `colors.rs` receives a new `complexity_penalty` parameter derived from the TDG grade. The grade-to-penalty mapping (A+=0.0 through F=1.0) follows the existing `grade_to_t()` scale inverted. The `compute_max_risk_raw()` function in `rects.rs` and all call sites for `compute_raw_risk()` are updated to pass the TDG grade. The `PmatReport.by_path` index provides the grade lookup by file path.

**Primary recommendation:** Implement in three sequential plans — risk model first (pure logic, easy to test), then +/- badges (rendering-only), then auto-diff (state + threading).

---

## Standard Stack

### Core (already in project — no new dependencies)

| Component | Version | Purpose | Notes |
|-----------|---------|---------|-------|
| egui/eframe | 0.31 | Rendering text badges on rects | `painter.text()` already used extensively |
| git2 | workspace dep | Reading `lines_added`/`lines_removed` from commits | Already used by `git_walker.rs` |
| crossbeam-channel | workspace dep | `maybe_spawn_*` thread pattern | Established pattern, no changes needed |

No new crate dependencies for this phase. All three features are pure logic and rendering changes within existing modules.

---

## Architecture Patterns

### Established Patterns (use exactly, don't invent)

**Pattern 1: Flag-based background thread spawning**
The entire phase follows the `*_requested` flag pattern:
```
AppState.{feature}_requested = true
→ draw_toolbar_panel() detects flag
→ maybe_spawn_{feature}_thread(app)
→ ScanMsg::{Feature}Ready(report)
→ poll_scan_messages() stores to state
```
The auto-diff uses this: set `git_diff_requested = true` in `apply_scan_reports()` or watcher handler after detecting an in-progress phase.

**Pattern 2: RenderContext pass-through**
`RenderContext` is built in `paint_render_frame()` and passed to `file_color()` and `draw_rects()`. To make `FileDiffData.lines_added/lines_removed` available to the badge renderer, it is accessed via `ctx.git_diff_report.as_ref()` — no new fields needed on RenderContext.

**Pattern 3: `draw_delta_arrow()` for per-rect overlays**
```rust
// Existing pattern in draw_file_rect():
if let Some(delta_report) = ctx.delta_report {
    if let Some(delta) = delta_report.by_file.get(r.path.as_str()) {
        draw_delta_arrow(dctx.painter, screen_rect, delta);
    }
}
```
The +/- badge follows the identical call pattern below `draw_delta_arrow()`. Both are gated by `lod_full`.

**Pattern 4: Directory aggregation**
```rust
// Existing in aggregate_dir_delta():
let children: Vec<&FileDiffData> = if dir_prefix.is_empty() {
    by_file.values().collect()
} else {
    by_file.iter()
        .filter(|(k, _)| k.starts_with(dir_prefix))
        .map(|(_, v)| v)
        .collect()
};
```
Badge directory aggregation uses the same filter — sum `lines_added` and `lines_removed` across children. Do NOT store an aggregate struct; compute the two u32 sums inline.

**Pattern 5: Grade-to-multiplier via `grade_to_t()`**
The complexity_penalty is `1.0 - grade_to_t(grade)` — directly inverts the existing 0.0-1.0 scale:
- A+ → grade_to_t = 1.0 → penalty = 0.0
- F   → grade_to_t = 0.0 → penalty = 1.0

This requires no new mapping function.

### File Modification Map

| File | Change |
|------|--------|
| `sentrux-core/src/renderer/colors.rs` | Add `complexity_penalty: f64` parameter to `compute_raw_risk()` |
| `sentrux-core/src/renderer/rects.rs` | Add `draw_diff_badge()`, call from `draw_file_rect()` and `draw_section_rect()`; update `compute_max_risk_raw()` call site |
| `sentrux-core/src/app/state.rs` | Add `auto_diff_active: bool` field |
| `sentrux-core/src/app/scanning.rs` | In `apply_scan_reports()`: detect InProgress phase, trigger auto-diff; in watcher rescan path: re-trigger if `auto_diff_active` |
| `sentrux-core/src/app/draw_panels.rs` | No change required — existing `git_diff_requested` flag handling already covers the auto-trigger path |

### Recommended Project Structure

No new modules. All changes are extensions of existing files.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Grade→multiplier mapping | New match table | `1.0 - grade_to_t(grade)` | `grade_to_t()` already perfectly covers the 0.0-1.0 range; inversion gives the penalty directly |
| Directory aggregation loop | New helper fn | Inline the children filter from `aggregate_dir_delta()` | The +/- badge sum needs just two u32 values, not a full `FileDeltaEntry`; a separate struct is wasteful |
| Finding in-progress phase | Linear scan with custom logic | `phases.iter().rposition()` or find last `InProgress` | Simple iterator, no custom data structure |
| Text badge layout | Custom layout math | `painter.text()` with `Align2::RIGHT_BOTTOM` | Consistent with all existing text rendering in `rects.rs` |

---

## Common Pitfalls

### Pitfall 1: Auto-diff overwriting user timeline selection
**What goes wrong:** If the user has already clicked a phase in the timeline and `auto_diff_active` fires again (e.g. on rescan), it resets `git_diff_window` and wipes `pre_timeline_color_mode`, corrupting the save/restore flow.
**Root cause:** The auto-switch uses the same `git_diff_window` + `color_mode` fields as user-initiated timeline clicks.
**How to avoid:** Guard the auto-switch behind `state.timeline_selection.is_none()`. If the user has made a selection, skip the auto-switch. Only fire auto-switch when: (1) `timeline_selection.is_none()` AND (2) `gsd_phase_report` contains an `InProgress` phase with a `commit_range`.
**Warning signs:** Tests show `pre_timeline_color_mode` overwritten during rescan.

### Pitfall 2: `compute_raw_risk()` call sites missing the new parameter
**What goes wrong:** `compute_raw_risk()` is called from two locations — `risk_color()` and `compute_max_risk_raw()`. Both must receive the grade. The max-risk normalization loop in `compute_max_risk_raw()` iterates over `GraphMetricsNode` entries, which only have basename-level names. TDG grade lookup requires path matching (via `PmatReport.by_path` or `by_basename`).
**Root cause:** `GraphMetricsNode.name` is a bare filename ("channels.rs"), not a full path. `PmatReport.by_path` uses full paths. The lookup strategy must be consistent between `risk_color()` and `compute_max_risk_raw()`.
**How to avoid:** Add a `by_basename` index to `PmatReport` (or reuse the existing `CoverageReport.by_basename` pattern). Alternatively, pass `Option<&str>` grade to `compute_raw_risk()`, defaulting to "unknown" (penalty=1.0) when no grade is found — this keeps false-alarm behavior for ungraded files.
**Warning signs:** Normalization produces wrong max_raw if grade lookup fails silently.

### Pitfall 3: Badge rendering at sub-threshold rect sizes
**What goes wrong:** The `draw_delta_arrow()` function already has a size guard (`rect.width() < 24.0 || rect.height() < 14.0`). The +/- badge has potentially wider text ("+1234 -567") and needs a wider minimum width.
**Root cause:** Badge text width varies with line count magnitude. A 12-character "+1234 -567" at 5px/char = 60px minimum.
**How to avoid:** Use `rect.width() < 60.0` as the badge visibility threshold. This is Claude's discretion per CONTEXT.md.
**Warning signs:** Badge text overflows into file name or clips at left edge.

### Pitfall 4: +0 -0 display decision
**What goes wrong:** Files touched by commits but with no net line changes (pure renames, whitespace reformatting) produce `lines_added=0, lines_removed=0` in `FileDiffData`. Showing "+0 -0" is noise.
**Root cause:** `FileDiffData` stores totals that can legitimately be zero when `commit_count > 0`.
**How to avoid:** Skip badge rendering when both `lines_added == 0` and `lines_removed == 0`. This is Claude's discretion per CONTEXT.md.

### Pitfall 5: Auto-diff on startup race condition
**What goes wrong:** App startup sequence is: `new()` → `apply_to(&mut state)` → first frame → `maybe_start_scan()`. The GSD phase report only arrives after scan completes, then `apply_scan_reports()` triggers `gsd_phase_requested`, then `GsdPhaseReady` fires. The auto-diff cannot happen at startup from prefs — it must wait for `GsdPhaseReady`.
**Root cause:** Prefs restore sets `root_path` but not `gsd_phase_report`. The phase report is always re-parsed fresh per scan.
**How to avoid:** The auto-diff trigger belongs in `ScanMsg::GsdPhaseReady` handler in `scanning.rs`. Do NOT try to detect InProgress phase in `prefs.apply_to()` or `new()`.

### Pitfall 6: Watcher-triggered auto-diff spamming
**What goes wrong:** The watcher fires `FileEvent` for every saved file during an AI coding session — potentially dozens per second. Each event currently debounces into a rescan (500ms). If the auto-diff re-triggers on every `GsdPhaseReady` (each rescan), this is fine since it follows the existing debounce path.
**Root cause:** The watcher debounce path: `pending_changes → flush_pending_changes → start_rescan → scan completes → apply_scan_reports → gsd_phase_requested → GsdPhaseReady → auto-diff check`.
**How to avoid:** The natural rescan pipeline already debounces. The auto-diff trigger in `GsdPhaseReady` fires at most once per rescan cycle, which is the correct rate.

---

## Code Examples

### Risk formula modification (colors.rs)

```rust
// Source: colors.rs — current signature
pub fn compute_raw_risk(pagerank: f64, coverage_pct: f64, clippy_count: u32) -> f64 {
    let pr = pagerank.clamp(0.0, 1.0);
    let uncovered = 1.0 - coverage_pct.clamp(0.0, 100.0) / 100.0;
    let lint_factor = 1.0 + (clippy_count as f64 + 1.0).ln() / 5.0;
    pr * uncovered * lint_factor
}

// Phase 6: add complexity_penalty parameter
pub fn compute_raw_risk(
    pagerank: f64,
    coverage_pct: f64,
    clippy_count: u32,
    complexity_penalty: f64,  // NEW: 0.0 (A+) to 1.0 (F)
) -> f64 {
    let pr = pagerank.clamp(0.0, 1.0);
    let uncovered = 1.0 - coverage_pct.clamp(0.0, 100.0) / 100.0;
    let lint_factor = 1.0 + (clippy_count as f64 + 1.0).ln() / 5.0;
    let penalty = complexity_penalty.clamp(0.0, 1.0);
    pr * penalty * uncovered * lint_factor
}
```

### Grade-to-penalty mapping (uses existing grade_to_t)

```rust
// In rects.rs color_by_risk() — derive penalty from grade
let tdg_grade = ctx.pmat_report
    .and_then(|r| r.by_path.get(path).map(|&i| r.tdg.files[i].grade.as_str()))
    .unwrap_or("unknown");
let complexity_penalty = 1.0 - crate::core::pmat_types::grade_to_t(tdg_grade) as f64;
// A+ → 1.0 - 1.0 = 0.0 (no penalty)
// F  → 1.0 - 0.0 = 1.0 (full penalty)
// unknown → 1.0 - 0.0 = 1.0 (conservative — unknown file treated as worst case)
```

### +/- badge rendering (rects.rs)

```rust
// In draw_file_rect() — after draw_delta_arrow():
if ctx.color_mode == crate::layout::types::ColorMode::GitDiff {
    if let Some(diff_report) = ctx.git_diff_report {
        if let Some(diff) = diff_report.by_file.get(r.path.as_str()) {
            draw_diff_badge(dctx.painter, screen_rect, diff.lines_added, diff.lines_removed);
        }
    }
}

fn draw_diff_badge(painter: &egui::Painter, rect: egui::Rect, added: u32, removed: u32) {
    if rect.width() < 60.0 || rect.height() < 14.0 {
        return;
    }
    // Skip +0 -0 (no line changes, possibly rename/whitespace)
    if added == 0 && removed == 0 {
        return;
    }
    let green = egui::Color32::from_rgb(80, 200, 80);
    let red   = egui::Color32::from_rgb(220, 60, 60);
    let font  = egui::FontId::monospace(8.0);

    // Draw from bottom-right inward
    let y = rect.bottom() - 2.0;
    let mut x = rect.right() - 2.0;

    // Removed (red) — rightmost
    if removed > 0 {
        let text = format!("-{}", removed);
        let w = text.len() as f32 * 5.0;
        painter.text(
            egui::pos2(x, y),
            egui::Align2::RIGHT_BOTTOM,
            &text,
            font.clone(),
            red,
        );
        x -= w + 3.0;
    }

    // Added (green) — left of removed
    if added > 0 {
        let text = format!("+{}", added);
        painter.text(
            egui::pos2(x, y),
            egui::Align2::RIGHT_BOTTOM,
            &text,
            font.clone(),
            green,
        );
    }
}
```

### Auto-diff trigger in GsdPhaseReady handler (scanning.rs)

```rust
// In poll_scan_messages() → ScanMsg::GsdPhaseReady handler:
ScanMsg::GsdPhaseReady(report) => {
    // ... existing milestone/phase building ...
    self.state.gsd_phase_report = Some(report);
    self.state.gsd_phase_running = false;

    // Auto-diff: if no user timeline selection exists, switch to GitDiff
    // showing the highest-numbered InProgress phase
    if self.state.timeline_selection.is_none() {
        let in_progress = self.state.gsd_phase_report.as_ref()
            .and_then(|r| {
                r.phases.iter()
                    .rposition(|p| p.status == PhaseStatus::InProgress)
                    .and_then(|idx| r.phases[idx].commit_range.clone())
            });
        if let Some((from_sha, _to_sha)) = in_progress {
            // Save color mode (same save/restore as timeline click)
            if self.state.pre_timeline_color_mode.is_none() {
                self.state.pre_timeline_color_mode = Some(self.state.color_mode);
            }
            self.state.color_mode = crate::layout::types::ColorMode::GitDiff;
            self.state.git_diff_window = crate::metrics::evo::git_walker::DiffWindow::CommitRange {
                from: from_sha,
                to: "HEAD".to_string(),
            };
            self.state.git_diff_requested = true;
            self.state.auto_diff_active = true;
        }
    }
    ctx.request_repaint();
}
```

### Directory +/- aggregation (rects.rs draw_section_rect)

```rust
// In draw_section_rect() — alongside existing aggregate_dir_delta():
if lod_full && ctx.color_mode == crate::layout::types::ColorMode::GitDiff {
    if let Some(diff_report) = ctx.git_diff_report {
        let dir_prefix = if r.path.is_empty() || r.path == "/" {
            String::new()
        } else {
            format!("{}/", r.path)
        };
        let (added, removed) = aggregate_dir_diff(&diff_report.by_file, &dir_prefix);
        if added > 0 || removed > 0 {
            draw_diff_badge(dctx.painter, screen_rect, added, removed);
        }
    }
}

fn aggregate_dir_diff(
    by_file: &std::collections::HashMap<String, crate::core::pmat_types::FileDiffData>,
    dir_prefix: &str,
) -> (u32, u32) {
    let iter: Box<dyn Iterator<Item = &crate::core::pmat_types::FileDiffData>> = if dir_prefix.is_empty() {
        Box::new(by_file.values())
    } else {
        Box::new(by_file.iter()
            .filter(|(k, _)| k.starts_with(dir_prefix))
            .map(|(_, v)| v))
    };
    iter.fold((0u32, 0u32), |(a, r), d| (a + d.lines_added, r + d.lines_removed))
}
```

---

## State of the Art

| Old Approach | New Approach | Change | Impact |
|--------------|--------------|--------|--------|
| Risk = PageRank × coverage_gap × lint_factor | Risk = PageRank × **complexity_penalty** × coverage_gap × lint_factor | Phase 6 | Simple hub files (mod.rs, A+ grade) no longer false-alarm as high risk |
| GitDiff color with no line-count context | GitDiff color + +/- badge at bottom-right | Phase 6 | User sees both "how hot" (color intensity) and "how much" (line counts) simultaneously |
| Manual timeline click to see phase changes | Auto-switch to InProgress phase on app open | Phase 6 | Zero-friction monitoring — open app, immediately see AI work from current phase |

---

## Open Questions

1. **Grade lookup in `compute_max_risk_raw()`**
   - What we know: `compute_max_risk_raw()` iterates `GraphMetricsNode` entries which have bare `name` (basename). `PmatReport.by_path` keys are full paths.
   - What's unclear: Does a `by_basename` index already exist on `PmatReport`? Grep shows `CoverageReport` has `by_basename` but `PmatReport` may not.
   - Recommendation: Check `pmat_types.rs` `PmatReport` struct. If no `by_basename`, the planner should add a plan step to add one, OR pass `Option<&str>` grade defaulting to "unknown" for unmatched nodes (conservative: full penalty for ungraded files, which is acceptable).

2. **Multiple InProgress phases edge case**
   - What we know: CONTEXT.md says "pick the highest numbered one" — this is Claude's discretion.
   - What's unclear: Whether multiple in-progress phases can realistically occur in the GSD workflow (it indicates overlapping uncommitted work).
   - Recommendation: Use `rposition()` on the phases slice to find the last `InProgress` entry — this naturally picks the highest-numbered one since phases are ordered by number.

3. **`auto_diff_active` flag lifecycle**
   - What we know: The flag tracks whether the auto-switch happened to distinguish from user clicks.
   - What's unclear: When should `auto_diff_active` be cleared? Options: (a) when user clicks timeline (overrides auto), (b) when phase becomes Completed, (c) never (until next GSD phase parse).
   - Recommendation: Clear `auto_diff_active` when `timeline_selection` is set by user action. The flag is informational only — the main logic uses `timeline_selection.is_none()` as the gate.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) |
| Config file | none — `cargo test` discovers tests inline |
| Quick run command | `cargo test -p sentrux-core 2>&1 | tail -20` |
| Full suite command | `cargo test --workspace 2>&1 | tail -30` |

### Phase Requirements → Test Map

| Feature | Behavior | Test Type | Automated Command | Location |
|---------|----------|-----------|-------------------|---------|
| Risk formula | A+ grade → near-zero risk even with high PageRank | unit | `cargo test -p sentrux-core compute_raw_risk` | `colors.rs` inline tests |
| Risk formula | F grade → same risk as old formula (penalty=1.0) | unit | `cargo test -p sentrux-core compute_raw_risk` | `colors.rs` inline tests |
| Risk formula | No regression: existing risk tests still pass | unit | `cargo test -p sentrux-core risk_color` | `colors.rs` existing tests |
| +/- badge | zero-count badge not shown (+0 -0 skipped) | unit | `cargo test -p sentrux-core diff_badge` | `rects.rs` inline tests |
| +/- badge | width threshold hides badge on small rects | unit | `cargo test -p sentrux-core diff_badge` | `rects.rs` inline tests |
| Dir aggregation | summed +/- across children correct | unit | `cargo test -p sentrux-core aggregate_dir_diff` | `rects.rs` inline tests |
| Auto-diff | InProgress phase with commit_range triggers auto-diff | unit | `cargo test -p sentrux-core auto_diff` | `scanning.rs` or new test |
| Auto-diff | No auto-diff when timeline_selection already set | unit | `cargo test -p sentrux-core auto_diff_selection_guard` | `scanning.rs` or new test |

### Sampling Rate
- **Per task commit:** `cargo test -p sentrux-core 2>&1 | tail -20`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -30`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

Wave 0 needs new test functions (not new files — tests live inline in Rust modules):

- [ ] `compute_raw_risk_aplus_near_zero` — covers risk model change (inside `colors.rs` `#[cfg(test)]`)
- [ ] `compute_raw_risk_f_grade_full_penalty` — covers risk model lower bound
- [ ] `draw_diff_badge_zero_count_skipped` — covers +0/-0 guard (inside `rects.rs` `#[cfg(test)]`)
- [ ] `aggregate_dir_diff_sums_children` — covers directory badge aggregation

*(No new test files needed — inline Rust test modules are the convention throughout this codebase)*

---

## Sources

### Primary (HIGH confidence)
- Direct source code inspection: `sentrux-core/src/renderer/colors.rs` — `compute_raw_risk()`, `grade_to_t()`, all call sites verified
- Direct source code inspection: `sentrux-core/src/renderer/rects.rs` — `draw_delta_arrow()`, `aggregate_dir_delta()`, `draw_file_rect()`, `draw_section_rect()`, `compute_max_risk_raw()` — full rendering pipeline verified
- Direct source code inspection: `sentrux-core/src/app/scanning.rs` — `apply_scan_reports()`, `GsdPhaseReady` handler, debounce/rescan flow verified
- Direct source code inspection: `sentrux-core/src/app/state.rs` — full `AppState` struct; confirmed no existing `auto_diff_active` field
- Direct source code inspection: `sentrux-core/src/app/draw_panels.rs` — full `draw_timeline_navigator()`, `maybe_spawn_git_diff_thread()`, flag handling flow verified
- Direct source code inspection: `sentrux-core/src/core/pmat_types.rs` — `FileDiffData.lines_added/lines_removed` field names confirmed; `grade_to_t()` scale confirmed
- Direct source code inspection: `sentrux-core/src/app/prefs.rs` — startup sequence confirmed; auto-diff must NOT fire from prefs restore
- Direct source code inspection: `sentrux-core/src/analysis/gsd_phase_adapter.rs` — `PhaseInfo.commit_range: Option<(String, String)>` confirmed; `PhaseStatus::InProgress` enum variant confirmed

### Secondary (MEDIUM confidence)
- `.planning/phases/06-ai-monitoring-ux/06-CONTEXT.md` — locked decisions, formula specification

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, all existing modules verified
- Architecture: HIGH — call sites, data structures, and patterns confirmed by source inspection
- Pitfalls: HIGH — identified through direct inspection of existing patterns and their edge cases
- Risk formula: HIGH — `grade_to_t()` scale and `compute_raw_risk()` signature both verified from source
- Auto-diff flow: HIGH — full `GsdPhaseReady` handler and `draw_timeline_navigator()` mutation code verified

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable codebase, no external dependencies changing)
