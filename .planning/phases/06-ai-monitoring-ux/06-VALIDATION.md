---
phase: 06
slug: ai-monitoring-ux
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 06 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[cfg(test)]` inline unit tests (cargo test) |
| **Config file** | none — workspace default |
| **Quick run command** | `cargo test -p sentrux-core --lib 2>&1 \| tail -20` |
| **Full suite command** | `cargo test -p sentrux-core 2>&1 \| tail -30` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p sentrux-core --lib 2>&1 | tail -20`
- **After every plan wave:** Run `cargo test -p sentrux-core 2>&1 | tail -30`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | risk model | unit | `cargo test -p sentrux-core compute_raw_risk` | ❌ W0 | ⬜ pending |
| 06-02-01 | 02 | 2 | +/- badge | unit | `cargo test -p sentrux-core diff_badge` | ❌ W0 | ⬜ pending |
| 06-02-02 | 02 | 2 | dir aggregation | unit | `cargo test -p sentrux-core aggregate_dir_diff` | ❌ W0 | ⬜ pending |
| 06-03-01 | 03 | 3 | auto-diff | unit | `cargo test -p sentrux-core auto_diff` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `colors.rs` — compute_raw_risk tests for A+ near-zero and F full-penalty
- [ ] `rects.rs` — diff badge zero-count guard and directory aggregation tests
- [ ] `scanning.rs` — auto-diff trigger and selection guard tests

*Existing test infrastructure (cargo test) covers framework needs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Auto-switch to GitDiff on open | auto-diff | Requires app launch with GSD project | Open app on sentrux, verify GitDiff mode activates |
| +/- badges render correctly | diff badge | Visual rendering | Switch to GitDiff, verify green/red counts on files |
| Risk colors change for mod.rs | risk model | Visual verification | Switch to Risk mode, verify mod.rs no longer red |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
