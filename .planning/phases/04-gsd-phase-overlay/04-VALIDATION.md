---
phase: 4
slug: gsd-phase-overlay
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-15
---

# Phase 4 — Validation Strategy

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
| 4-01-01 | 01 | 1 | GSDP-01,03 | unit | `cargo test -p sentrux-core gsd_phase` | Wave 0 | ⬜ pending |
| 4-01-02 | 01 | 1 | GSDP-01 | unit | `cargo test -p sentrux-core color_mode_gsd_phase` | Wave 0 | ⬜ pending |
| 4-02-01 | 02 | 2 | GSDP-04 | unit | `cargo test -p sentrux-core gsd_phase_pipeline` | Wave 0 | ⬜ pending |
| 4-03-01 | 03 | 3 | GSDP-02,05 | manual | Hover file, verify tooltip | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] GSD phase types (GsdPhaseReport, FilePhaseInfo, PhaseStatus)
- [ ] Parser for PLAN.md frontmatter and SUMMARY.md key-files
- [ ] ColorMode::GsdPhase variant
- [ ] color_by_gsd_phase dispatch function

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Files colored by phase state | GSDP-01 | GUI rendering | Switch to GsdPhase mode, verify completed=green, in-progress=amber, planned=blue |
| Completed vs planned distinct | GSDP-02 | Visual distinction | Compare colors visually |
| Hover shows phase info | GSDP-05 | GUI tooltip | Hover a colored file, verify phase number/name/goal in tooltip |
| Phase navigator panel | CONTEXT | GUI interaction | Click phases in navigator, verify GitDiff window updates |
| Unassociated files muted | GSDP-04 | Visual check | Verify files not in any phase are muted gray |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
