---
phase: 05
slug: improve-time-alignment
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-15
---

# Phase 05 — Validation Strategy

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
| 05-01-01 | 01 | 1 | TBD | unit | `cargo test -p sentrux-core snapshot_writer` | ❌ W0 | ⬜ pending |
| 05-01-02 | 01 | 1 | TBD | unit | `cargo test -p sentrux-core timeline_delta` | ❌ W0 | ⬜ pending |
| 05-02-01 | 02 | 2 | TBD | unit | `cargo test -p sentrux-core timeline_widget` | ❌ W0 | ⬜ pending |
| 05-02-02 | 02 | 2 | TBD | unit | `cargo test -p sentrux-core time_ticks` | ❌ W0 | ⬜ pending |
| 05-02-03 | 02 | 2 | TBD | unit | `cargo test -p sentrux-core milestone_visibility` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `sentrux-core/src/analysis/snapshot_writer.rs` — snapshot write/load/prune tests
- [ ] `sentrux-core/src/app/timeline_widget.rs` — equal_segment_widths tests
- [ ] `sentrux-core/src/analysis/timeline_delta.rs` — FileDeltaEntry computation tests

*Existing test infrastructure (cargo test) covers framework needs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Hierarchical bar renders correctly | TBD | Visual UI layout | Run app, switch to GSD Phase mode, verify 3-tier bar |
| Click-to-zoom filters treemap | TBD | Interactive behavior | Click phase segment, verify treemap re-colors |
| Time ticks auto-scale | TBD | Visual scaling | Zoom in/out on timeline, verify tick granularity changes |
| Delta arrows render on nodes | TBD | Visual overlay | Select past range, verify ↑↓ arrows on changed files |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
