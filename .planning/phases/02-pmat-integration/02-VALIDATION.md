---
phase: 2
slug: pmat-integration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 2 — Validation Strategy

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
| 2-01-01 | 01 | 1 | PMAT-01 | unit | `cargo test -p sentrux-core pmat_adapter` | Wave 0 | ⬜ pending |
| 2-01-02 | 01 | 1 | PMAT-01 | unit | `cargo test -p sentrux-core pmat_not_found` | Wave 0 | ⬜ pending |
| 2-01-03 | 01 | 1 | PMAT-02 | unit | `cargo test -p sentrux-core grade_to_display` | Wave 0 | ⬜ pending |
| 2-02-01 | 02 | 2 | PMAT-04 | unit | `cargo test -p sentrux-core tdg_grade_color` | Wave 0 | ⬜ pending |
| 2-02-02 | 02 | 2 | PMAT-07 | unit | `cargo test -p sentrux-core color_mode_variants` | Wave 0 | ⬜ pending |
| 2-02-03 | 02 | 2 | serde compat | unit | `cargo test -p sentrux-core color_mode_serde_compat` | Wave 0 | ⬜ pending |
| 2-03-01 | 03 | 3 | PMAT-05 | unit | `cargo test -p sentrux-core repo_score_parse` | Wave 0 | ⬜ pending |
| 2-04-01 | 04 | 4 | CLEN-04 | compile | `cargo build --workspace` | Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `sentrux-core/src/core/pmat_types.rs` — PMAT data types, grade parsing, color mapping
- [ ] `sentrux-core/src/analysis/pmat_adapter.rs` — subprocess invocation + JSON parsing
- [ ] Unit tests for grade string mapping (`APLus` → `A+`)
- [ ] Unit tests for subprocess not found (graceful error)
- [ ] Unit tests for ColorMode serde backward compat

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| TDG grade badges visible on treemap nodes | PMAT-03 | Requires GUI rendering | Open project, verify badges appear on large nodes |
| TDG as default color mode on project open | PMAT-04 | Requires GUI state | Open fresh project, verify treemap shows TDG colors |
| Health panel shows PMAT score | PMAT-05 | Requires GUI panel | Open project, check health panel content |
| File detail shows TDG breakdown | PMAT-06 | Requires GUI interaction | Click file, verify component scores shown |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
