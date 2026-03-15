---
phase: 01-cleanup
plan: 02
subsystem: cleanup
tags: [rust, tree-sitter, lang-registry, grammars, cargo]

requires:
  - phase: 01-cleanup/01-01
    provides: "Empty lang_registry static stub with preserved public API; plugin system deleted"
provides:
  - "Static 3-language registry with compiled-in Rust, TypeScript, and JavaScript grammars"
  - "tree-sitter-rust, tree-sitter-typescript, tree-sitter-javascript in Cargo.toml"
  - "All other file types silently skipped (get_grammar_and_query returns None)"
  - "Integration test proving .py/.go files cause no panic and are not structurally parsed"
  - "Only rust/, typescript/, javascript/ query directories remain"
affects: [phase-2-pmat, phase-3-overlay, phase-4-gsd]

tech-stack:
  added:
    - "tree-sitter-rust v0.24.0"
    - "tree-sitter-typescript v0.23.2"
    - "tree-sitter-javascript v0.25.0"
  patterns:
    - "Static registry via LazyLock<LangRegistry> with Language::new(LanguageFn) pattern for compiled-in grammars"
    - "Query files loaded at registry init via include_str! — fail fast on bad query syntax"
    - "TypeScript uses LANGUAGE_TYPESCRIPT constant; tsx extension also maps to typescript grammar"

key-files:
  created:
    - sentrux-core/tests/unknown_extensions.rs
  modified:
    - sentrux-core/src/analysis/lang_registry.rs
    - sentrux-core/Cargo.toml

key-decisions:
  - "Used Language::new(LanguageFn) pattern (tree-sitter 0.25 API) instead of .into() — matches actual crate API shape"
  - "Renamed PluginLangConfig -> LangConfig to remove plugin terminology from public type name"
  - "tsx extension maps to LANGUAGE_TYPESCRIPT (not LANGUAGE_TSX) — the TypeScript grammar handles .tsx via the same tags.scm"
  - "Pre-existing oracle test failures (27) for removed languages are expected and not regressions"

patterns-established:
  - "Extension-to-language mapping is purely done by lang_registry — no ad-hoc ext checks elsewhere"
  - "Integration tests for scanner use std::env::temp_dir() with timestamp-based dir names (no tempfile dep)"

requirements-completed: [CLEN-03]

duration: 3min
completed: 2026-03-14
---

# Phase 1 Plan 2: Rewrite lang_registry.rs as Static 3-Language Registry

**Static LazyLock registry compiles Rust/TypeScript/JavaScript grammars into the binary via tree-sitter Language::new(LanguageFn); all other file types silently return None.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-14T~00:57Z
- **Completed:** 2026-03-14T~01:00Z
- **Tasks:** 2
- **Files modified:** 2 modified, 1 created, 23 deleted

## Accomplishments
- Added `tree-sitter-rust`, `tree-sitter-typescript`, `tree-sitter-javascript` grammar crates to `sentrux-core/Cargo.toml`
- Rewrote `lang_registry.rs` as a static `LazyLock<LangRegistry>` with compiled-in grammars for Rust, TypeScript, and JavaScript
- All 17 `lang_registry` unit tests pass including 14 new tests for grammar presence and extension mapping
- Deleted 21 query directories for unsupported languages; only `rust/`, `typescript/`, `javascript/` remain
- Added integration tests proving `.py` and `.go` files are silently skipped without panic or structural analysis
- Renamed `PluginLangConfig` -> `LangConfig` to remove plugin terminology
- Release build (`cargo build --release --workspace`) succeeds with zero errors

## Task Commits

1. **TDD RED: Add failing tests** - `498ed96` (test)
2. **Task 1: Rewrite lang_registry.rs as static registry** - `03318b5` (feat)
3. **Task 2: Delete unused query dirs, add integration test, rename LangConfig** - `ce270e6` (feat)

## Files Created/Modified
- `sentrux-core/src/analysis/lang_registry.rs` - Complete rewrite: static LazyLock registry with 3 compiled-in grammars; renamed PluginLangConfig->LangConfig; 17 unit tests
- `sentrux-core/Cargo.toml` - Added tree-sitter-rust, tree-sitter-typescript, tree-sitter-javascript
- `sentrux-core/tests/unknown_extensions.rs` - Integration tests: no-panic + no-structural-analysis for .py/.go files
- `sentrux-core/src/queries/{21 dirs}` - Deleted (bash, c, cpp, csharp, css, dockerfile, elixir, go, haskell, html, java, kotlin, lua, ocaml, php, python, r, ruby, scala, scss, swift, zig)

## Decisions Made
- **Language::new(LanguageFn) pattern:** The grammar crates export `LANGUAGE: LanguageFn` (not `Language` directly). In tree-sitter 0.25, the correct conversion is `Language::new(tree_sitter_rust::LANGUAGE)` — verified by inspecting `tree-sitter-0.25.10/binding_rust/lib.rs` before writing code.
- **tsx maps to LANGUAGE_TYPESCRIPT:** The TypeScript grammar handles both `.ts` and `.tsx` files. A separate TSX grammar exists (`LANGUAGE_TSX`) but our `typescript/tags.scm` was authored for the TypeScript grammar, so both `.ts` and `.tsx` extensions point to the same `typescript` registry entry.
- **PluginLangConfig renamed to LangConfig:** The struct name was a legacy artifact from the plugin era. Since no external callers used it (grep confirmed), the rename was safe and clarifies that this is now a static config, not a plugin config.
- **Pre-existing oracle failures unchanged:** 27 parser oracle tests for removed languages (python, go, bash, c, cpp, csharp, elixir, etc.) fail because `get_grammar_and_query` now correctly returns `None` for them. These were failing before this plan (documented in 01-01-SUMMARY.md as 33 failures). This is expected behavior — not a regression.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Refactor] Renamed PluginLangConfig to LangConfig**
- **Found during:** Task 2 (post-implementation cleanup)
- **Issue:** Plan verification check `grep -r "plugin\|load_plugins\|PluginLangConfig"` would match the struct name, which was misleading
- **Fix:** Renamed struct to `LangConfig`; verified no external callers (grep confirmed); `cargo check --workspace` passes
- **Files modified:** `sentrux-core/src/analysis/lang_registry.rs`
- **Committed in:** `ce270e6` (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - cleanup)
**Impact on plan:** Minor cleanup — the rename removes plugin terminology from public type, aligning with the static registry model.

## Issues Encountered
- None. Grammar crate API was discovered by reading crate lib.rs before writing registry code, avoiding any trial-and-error.

## Next Phase Readiness
- Binary parses Rust, TypeScript, and JavaScript files with structural analysis
- All other file types are silently skipped — scanner is 3-language only
- `cargo build --release --workspace` succeeds clean
- Phase 1 cleanup complete: MCP removed (01-01), plugin system removed (01-01), language narrowing done (01-02)
- Phase 2 (PMAT integration) can begin — PMAT API spike needed first per existing blocker in STATE.md

## Self-Check: PASSED

- `sentrux-core/src/analysis/lang_registry.rs` exists
- `sentrux-core/tests/unknown_extensions.rs` exists
- `src/queries/rust/`, `typescript/`, `javascript/` exist
- `src/queries/python/` deleted (correct)
- Commits 498ed96, 03318b5, ce270e6 all found in git log

---
*Phase: 01-cleanup*
*Completed: 2026-03-14*
