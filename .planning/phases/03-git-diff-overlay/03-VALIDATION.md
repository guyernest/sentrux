---
phase: 3
slug: git-diff-overlay
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-15
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | none (standard Cargo test runner) |
| **Quick run command** | `cargo test -p sentrux-core --lib 2>&1 \| tail -20` |
| **Full suite command** | `cargo test --workspace 2>&1 \| tail -30` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p sentrux-core --lib 2>&1 | tail -20`
- **After every plan wave:** Run `cargo test --workspace 2>&1 | tail -30`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 3-01-01 | 01 | 1 | GDIT-01,02 | unit | `cargo test -p sentrux-core color_mode_gitdiff diff_window` | Wave 0 | ⬜ pending |
| 3-01-02 | 01 | 1 | GDIT-03,04 | unit | `cargo test -p sentrux-core git_diff_intensity git_diff_color` | Wave 0 | ⬜ pending |
| 3-02-01 | 02 | 2 | GDIT-05 | unit | `cargo test -p sentrux-core git_diff_thread` | Wave 0 | ⬜ pending |
| 3-03-01 | 03 | 3 | OVRL-01,02,03 | manual | Run sentrux, verify toolbar + legend + persistence | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `sentrux-core/src/core/git_diff_types.rs` — DiffWindow, FileDiffData, GitDiffReport types
- [ ] `sentrux-core/src/renderer/colors.rs` — git_diff_intensity_color, git_diff_new_file_color
- [ ] `sentrux-core/src/layout/types.rs` — ColorMode::GitDiff variant

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Window selector visible only in GitDiff mode | OVRL-01 | GUI interaction | Switch to GitDiff mode, verify presets appear; switch away, verify gone |
| Color legend shows gradient + new-file swatch | OVRL-02 | GUI rendering | In GitDiff mode, verify legend renders below toolbar |
| Selected window persists across restart | OVRL-03 | Requires app restart | Set window to 1w, close+reopen, verify 1w selected |
| Metric deltas in detail panel | CONTEXT | GUI + data | Click changed file in GitDiff mode, verify TDG/coverage/clippy deltas shown |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
