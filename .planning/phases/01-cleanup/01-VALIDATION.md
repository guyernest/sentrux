---
phase: 1
slug: cleanup
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | none (standard Cargo test runner) |
| **Quick run command** | `cargo test -p sentrux-core --lib 2>&1 \| tail -5` |
| **Full suite command** | `cargo test --workspace 2>&1 \| tail -20` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo check --workspace`
- **After every plan wave:** Run `cargo test --workspace 2>&1 | tail -20`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 1 | CLEN-01 | build smoke | `cargo build -p sentrux-bin 2>&1 \| grep -c error` (must be 0) | Wave 0 | ⬜ pending |
| 1-01-02 | 01 | 1 | CLEN-02 | unit | `cargo test -p sentrux-core lang_registry` | ✅ exists | ⬜ pending |
| 1-01-03 | 01 | 1 | CLEN-03 | unit | `cargo test -p sentrux-core lang_registry` | ✅ partially | ⬜ pending |
| 1-01-04 | 01 | 1 | CLEN-04 | build check | `cargo build -p sentrux-core 2>&1 \| grep -c "dead_code"` | Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `sentrux-core/src/analysis/lang_registry.rs` — update `test_registry_loads` for static registry; add tests for `get_grammar_and_query("rust")` (must succeed), `get_grammar_and_query("python")` (must return None), `detect_lang_from_ext("rs")` → "rust", `detect_lang_from_ext("go")` → "unknown"
- [ ] Integration test: scan directory with `.py` and `.go` files, verify no panic and no error in `ScanResult`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `sentrux mcp` subcommand rejected | CLEN-01 | CLI argument rejection | Run `sentrux mcp` and verify it errors with "unknown subcommand" |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
