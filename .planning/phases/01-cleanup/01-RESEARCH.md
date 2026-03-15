# Phase 1: Cleanup - Research

**Researched:** 2026-03-14
**Domain:** Rust codebase subtraction — removing MCP server, plugin system, and narrowing language support
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLEN-01 | MCP server mode is removed from sentrux (PMAT provides MCP) | Full inventory of MCP entry points in `sentrux-bin/src/main.rs` and `sentrux-core/src/app/mcp_server/`; removal is self-contained |
| CLEN-02 | Plugin system (runtime tree-sitter grammar loading) is removed | Full inventory of plugin module, `lang_registry.rs`, auto-install code in `main.rs`; requires replacing plugin-based `LangRegistry` with a static built-in registry |
| CLEN-03 | Language support is narrowed to Rust, TypeScript, and JavaScript only | `lang_registry.rs` currently defers 100% to runtime plugins; must become a static registry with three hard-coded entries; all other `.scm` query files except `rust/`, `typescript/`, `javascript/` can be removed |
| CLEN-04 | Unused analysis code (sentrux's own metrics engine) is removed after PMAT replaces it | Per roadmap and ARCHITECTURE.md: this is explicitly Phase 2 work. CLEN-04 is **not in scope for Phase 1** — the metrics engine is actively used by `sentrux check` and `sentrux gate` CLI commands which must keep working until PMAT replaces them. This must be clarified and confirmed. |
</phase_requirements>

---

## Summary

Phase 1 is a pure subtractive refactor. The goal is to delete everything sentrux will not carry into the PMAT-powered future: the MCP server, the runtime plugin system, and all language grammars beyond Rust/TypeScript/JavaScript. Nothing new is built — every task is a deletion with a compilation check at the end.

The four requirements break cleanly into three distinct subsystems to remove plus a language list to narrow. CLEN-01 (MCP) and CLEN-02 (plugins) are fully self-contained subsystems; deleting them requires updating their callers in `sentrux-bin/src/main.rs` and removing the modules. CLEN-03 (language narrowing) is a forced consequence of CLEN-02: once the plugin system is gone, the `LangRegistry` must be rewritten as a static registry that compiles Rust/TS/JS grammars directly into the binary using `include_str!` for query files and the existing `tree-sitter-rust`, `tree-sitter-typescript`, and `tree-sitter-javascript` crates.

CLEN-04 requires careful interpretation. The ROADMAP and ARCHITECTURE documents describe the metrics engine removal as happening "after PMAT replaces it" (Phase 2). The CLI commands `sentrux check` and `sentrux gate` both call `metrics::compute_health` and `metrics::arch::compute_arch` — removing these in Phase 1 would break the CLI before PMAT is available. The safe interpretation is: CLEN-04 authorizes **marking** the metrics engine as scheduled for removal (e.g., `#[deprecated]` or a code comment), while the actual deletion lands in Phase 2 after PMAT is wired in. This is the only reading consistent with "after PMAT replaces it" in the requirements.

**Primary recommendation:** Execute CLEN-01, CLEN-02, and CLEN-03 in full. Treat CLEN-04 as out-of-scope for Phase 1 (or confirm with user). The binary must build clean and the CLI must remain functional after Phase 1.

---

## Standard Stack

### Core (already present — no new dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tree-sitter | 0.25 | Parse tree infrastructure | Already in Cargo.toml |
| tree-sitter-rust | (to be added) | Compiled-in Rust grammar | Replaces plugin-loaded grammar |
| tree-sitter-typescript | (to be added) | Compiled-in TS/JS grammar | Replaces plugin-loaded grammar |

### Dependencies to Remove
| Library | Currently Used For | Safe to Remove After |
|---------|--------------------|---------------------|
| `libloading` | Dynamic library loading for plugins | CLEN-02 complete |
| `dirs` | `~/.sentrux/plugins/` home dir lookup | CLEN-02 complete |

**Note on `dirs` removal:** `dirs` is only used in `sentrux-core/src/analysis/plugin/loader.rs:40` (`dirs::home_dir()`). After the plugin module is removed, the entire `dirs` dependency can be dropped from `sentrux-core/Cargo.toml`.

**Note on `libloading` removal:** `libloading` is only used in `sentrux-core/src/analysis/plugin/loader.rs:167-192`. After CLEN-02, it can be removed from `sentrux-core/Cargo.toml`.

### Grammars to Add to Cargo.toml
```toml
# sentrux-core/Cargo.toml — add after CLEN-02
tree-sitter-rust = "0.23"          # or latest compatible with tree-sitter 0.25
tree-sitter-typescript = "0.23"    # covers both .ts and .tsx
tree-sitter-javascript = "0.23"    # covers .js, .jsx, .mjs, .cjs
```

**Confidence note (MEDIUM):** Exact crate versions require verification against `tree-sitter` 0.25 ABI. The `tree-sitter-language` API changed between 0.22 and 0.23 — verify compatibility before pinning.

**Installation (after CLEN-02):**
```bash
cargo add tree-sitter-rust tree-sitter-typescript tree-sitter-javascript --manifest-path sentrux-core/Cargo.toml
cargo remove libloading dirs --manifest-path sentrux-core/Cargo.toml
```

---

## Architecture Patterns

### Recommended Project Structure After Phase 1

```
sentrux-core/src/
├── analysis/
│   ├── lang_registry.rs     # REWRITTEN: static 3-language registry (no plugin loading)
│   ├── parser/              # UNCHANGED: still uses LangRegistry API
│   ├── scanner/             # UNCHANGED
│   ├── graph/               # UNCHANGED
│   ├── resolver/            # UNCHANGED
│   ├── git.rs               # UNCHANGED
│   └── entry_points.rs      # UNCHANGED
│   # plugin/ directory: DELETED entirely
├── queries/
│   ├── rust/tags.scm        # KEPT
│   ├── typescript/tags.scm  # KEPT
│   ├── javascript/tags.scm  # KEPT
│   # All other language directories: DELETED
├── app/
│   # mcp_server/ directory: DELETED entirely
│   └── ... (all other app files: UNCHANGED)
└── metrics/                 # UNCHANGED (until Phase 2)
```

### Pattern 1: Static LangRegistry (replacing plugin loader)

**What:** After CLEN-02, `lang_registry.rs` must be rewritten. Instead of a `LazyLock<LangRegistry>` that calls `load_all_plugins()` at runtime, it becomes a static registry initialized from three compiled-in grammars.

**When to use:** The existing `LangRegistry` API (`get_by_ext`, `get_grammar_and_query`, `detect_lang_from_ext`, `all_extensions`) must be preserved exactly — the parser and scanner call these functions. The implementation changes; the interface stays.

**Approach (no tree-sitter-language struct changes required):**

```rust
// Source: sentrux-core/src/analysis/lang_registry.rs (rewritten)
use tree_sitter::{Language, Query};
use std::sync::LazyLock;

pub struct LangConfig {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub grammar: Language,
    pub query: Query,
}

static RUST_CONFIG: LazyLock<LangConfig> = LazyLock::new(|| {
    let grammar = tree_sitter_rust::LANGUAGE.into();
    let query = Query::new(&grammar, include_str!("../queries/rust/tags.scm"))
        .expect("Rust query compile failed");
    LangConfig { name: "rust", extensions: &["rs"], grammar, query }
});

static TS_CONFIG: LazyLock<LangConfig> = LazyLock::new(|| {
    let grammar = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let query = Query::new(&grammar, include_str!("../queries/typescript/tags.scm"))
        .expect("TypeScript query compile failed");
    LangConfig { name: "typescript", extensions: &["ts", "tsx"], grammar, query }
});

static JS_CONFIG: LazyLock<LangConfig> = LazyLock::new(|| {
    let grammar = tree_sitter_javascript::LANGUAGE.into();
    let query = Query::new(&grammar, include_str!("../queries/javascript/tags.scm"))
        .expect("JavaScript query compile failed");
    LangConfig { name: "javascript", extensions: &["js", "jsx", "mjs", "cjs"], grammar, query }
});

pub fn get_grammar_and_query(name: &str) -> Option<(&'static Language, &'static Query)> {
    match name {
        "rust" => Some((&RUST_CONFIG.grammar, &RUST_CONFIG.query)),
        "typescript" => Some((&TS_CONFIG.grammar, &TS_CONFIG.query)),
        "javascript" => Some((&JS_CONFIG.grammar, &JS_CONFIG.query)),
        _ => None,
    }
}
```

**Note on tree-sitter 0.25 API:** The `LANGUAGE` constant (type `LanguageFn`) must be converted to `Language` via `.into()`. This is the current pattern as of tree-sitter 0.24+. Verify against actual crate re-exports before coding.

### Pattern 2: MCP Removal (in `sentrux-bin/src/main.rs`)

**What:** Remove the `Mcp` subcommand, the `--mcp` hidden flag, the `run_mcp_server` call, and the import of `app::mcp_server`.

**Exact changes in `main.rs`:**
1. Remove `Command::Mcp` variant (line 76-77 area)
2. Remove `--mcp` hidden arg on `Cli` struct (lines 44-46)
3. Remove the `if cli.mcp_flag` block (lines 132-135)
4. Remove the `Some(Command::Mcp)` match arm (lines 144-147)
5. Remove the `Plugin` subcommand and `PluginAction` enum entirely
6. Remove `auto_install_plugins_if_needed()` call and function body
7. Remove all `Plugin` match arm and `run_plugin` function

**What stays in `main.rs`:** `run_gui`, `run_check`, `run_gate`, `cli_scan_limits`, GPU probing, `version_string`.

### Pattern 3: Scanner Language Filtering (CLEN-03)

**What:** The scanner calls `detect_lang(path)` → `lang_registry::detect_lang_from_ext(ext)`. After the static registry, non-Rust/TS/JS files will get `"unknown"` from `get_by_ext`. The scanner currently includes all files regardless of language. Files with `lang == "unknown"` are already line-counted but not structurally parsed.

**Verification needed:** Confirm that the scanner silently skips files with unknown languages during the parsing phase (CLEN-03 success criterion). Looking at `scanner/common.rs:detect_lang` and the parser call site — if `lang_registry::get_grammar_and_query(lang)` returns `None`, `parse_file` returns `None`, which is already handled gracefully. No additional error handling needed. The "silently skips all other file types without error" criterion is met by the existing `Option` propagation.

**The `detect_lang_from_ext` fallback table** in the current `lang_registry.rs` (lines 144-159) lists languages like `json`, `toml`, `yaml`, etc. as display-only (non-parseable). These can be kept or trimmed — they don't affect the success criteria for CLEN-03 since they already produce `"unknown"` or named-but-not-parsed entries.

### Anti-Patterns to Avoid

- **Deleting query files before removing the include_str! references:** Will cause compile errors. Always remove `include_str!` uses before deleting the `.scm` files (or delete both in the same commit).
- **Removing `libloading` from Cargo.toml before deleting all plugin code:** Cargo will report unresolved imports. Remove the code first, then the dependency.
- **Leaving dead `use` statements:** Rust dead-code lints will surface them. Check that no `use sentrux_core::app::mcp_server` or `use crate::analysis::plugin` remains after deletion.
- **Removing `pub use evo as evolution`** alias before migrating `mcp_server/handlers_evo.rs`: Since the MCP server is being deleted, the alias can be removed at the same time as the MCP module. The CONCERNS.md notes this alias exists specifically for `mcp_handlers_evo.rs` — which will be gone.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Compile-in tree-sitter grammars | Custom grammar loading | `tree-sitter-rust`, `tree-sitter-typescript`, `tree-sitter-javascript` crates | These crates are the official way to embed grammars; each exports a `LANGUAGE` constant |
| Extension → language mapping | Custom HashMap init | Static match arms or `phf` | Three languages = trivial match statement, no need for a map |

---

## Common Pitfalls

### Pitfall 1: CLEN-04 Scope Ambiguity
**What goes wrong:** The requirements say "unused analysis code... is removed after PMAT replaces it." If Phase 1 deletes `metrics/grading.rs`, `metrics/stability.rs`, and `metrics/whatif.rs`, the `sentrux check` and `sentrux gate` CLI commands break — they call `compute_health` which uses all of these.
**Why it happens:** CLEN-04 is listed in Phase 1 requirements, but its own description says "after PMAT replaces it" — that happens in Phase 2.
**How to avoid:** Treat CLEN-04 as a Phase 2 deliverable. If the planner must address it in Phase 1, the safe action is to remove only `metrics/whatif` (used only in the GUI `whatif_display.rs` panel and the MCP server — both gone after CLEN-01 and cleanup), not the entire metrics engine.
**Warning signs:** Any plan task that deletes `metrics/mod.rs`, `metrics/grading.rs`, or `metrics/stability.rs` in Phase 1 is out of scope.

### Pitfall 2: `queries/` directory contains `include_str!` macros
**What goes wrong:** Deleting non-Rust/TS/JS `.scm` files while old `include_str!` references exist causes compile errors.
**Why it happens:** The original `lang_registry.rs` was ALL plugin-based — there are no `include_str!` calls in it currently. But the rewritten static registry will use `include_str!` for Rust/TS/JS. The risk is the reverse: accidentally referencing a `.scm` file path that was deleted.
**How to avoid:** Delete all non-Rust/TS/JS query directories. Verify the three remaining `.scm` files are referenced correctly by the new `include_str!` paths in the rewritten `lang_registry.rs`.

### Pitfall 3: Tree-sitter Grammar API Version Mismatch
**What goes wrong:** `tree-sitter` 0.25 changed the grammar constant API. The `LANGUAGE` exported by newer grammar crates may be of type `LanguageFn` (a function pointer), not `Language` directly.
**Why it happens:** tree-sitter upstream is actively evolving its Rust bindings.
**How to avoid:** After `cargo add tree-sitter-rust tree-sitter-typescript tree-sitter-javascript`, run `cargo check` immediately to see if the grammar constants need `.into()`, direct call `()`, or `Language::new()` wrapping. Do not assume the API shape from documentation — verify from the generated `.d.ts` or `cargo doc`.

### Pitfall 4: `sentrux-pro` private crate
**What goes wrong:** The ARCHITECTURE.md notes a private `sentrux-pro` crate. If it imports `app::mcp_server::McpState` or `ToolDef`/`ToolRegistry` types, removing the MCP module breaks that crate.
**Why it happens:** `sentrux-pro` is not in this repository but may depend on `sentrux-core`'s public MCP types.
**How to avoid:** Before deleting `app/mcp_server`, confirm whether `sentrux-pro` uses any MCP types. If it does, those types must either be kept (in a stripped form) or `sentrux-pro` must be updated simultaneously. This is flagged as LOW confidence because the private crate is not inspectable here.

### Pitfall 5: `auto_install_plugins_if_needed` runs before GPU init
**What goes wrong:** The plugin auto-installer is called on every binary startup (line 124 of `main.rs`) before GPU probing. After CLEN-02, leaving a stub that always exits early is harmless but the whole function must be removed cleanly.
**How to avoid:** Delete both the `auto_install_plugins_if_needed()` call site AND the entire function body. Do not leave an empty stub.

---

## Code Examples

Verified patterns from the actual source:

### MCP entry points to remove
```rust
// sentrux-bin/src/main.rs — all of these must be deleted:

// 1. Hidden --mcp flag on Cli struct (lines 44-46)
#[arg(long = "mcp", hide = true)]
mcp_flag: bool,

// 2. Flag dispatch in main() (lines 132-135)
if cli.mcp_flag {
    app::mcp_server::run_mcp_server(None);
    return Ok(());
}

// 3. Mcp subcommand variant in Command enum (lines 75-76)
/// Start the MCP (Model Context Protocol) server
Mcp,

// 4. Match arm in main() (lines 144-147)
Some(Command::Mcp) => {
    app::mcp_server::run_mcp_server(None);
    Ok(())
}
```

### Plugin auto-install to remove
```rust
// sentrux-bin/src/main.rs — remove from main():
auto_install_plugins_if_needed();  // line 124

// And remove the entire function (lines 672-737):
fn auto_install_plugins_if_needed() { ... }
```

### Plugin subcommand to remove
```rust
// Remove Command::Plugin variant and entire PluginAction enum
// Remove run_plugin() function (lines 338-546)
// Remove sentrux_core::analysis::plugin imports
```

### Module deletions
```
# Entire directories to delete:
sentrux-core/src/app/mcp_server/          # CLEN-01
sentrux-core/src/analysis/plugin/         # CLEN-02
sentrux-core/src/queries/<all except rust, typescript, javascript>/  # CLEN-03
```

### Module declarations to remove from mod.rs files
```rust
// In sentrux-core/src/app/mod.rs — remove:
pub mod mcp_server;

// In sentrux-core/src/analysis/mod.rs — remove:
pub mod plugin;

// In sentrux-core/src/metrics/mod.rs — remove (CLEN-04, Phase 2 only):
pub mod whatif;    // Only safe to remove if whatif_display.rs is also removed
pub use evo as evolution;  // Safe to remove once mcp_server/ is gone
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Compiled-in tree-sitter grammars | Runtime plugin loading | Original design | Plugin loading was designed first; Phase 1 reverts to compiled-in |
| `pub mod mcp_server` | (deleted) | Phase 1 | Callers in `sentrux-pro` must be updated |

**Deprecated/outdated after Phase 1:**
- `lang_registry.rs:PluginLangConfig` struct — replaced by static `LangConfig`
- `lang_registry.rs:LangRegistry:load_plugins()` — replaced by static initialization
- `metrics/mod.rs:pub use evo as evolution` alias — safe to remove once MCP server is gone

---

## Open Questions

1. **CLEN-04 scope in Phase 1**
   - What we know: CLEN-04 says "removed after PMAT replaces it"; Phase 2 is "PMAT Integration"; `sentrux check` and `sentrux gate` actively use `compute_health` and `arch::compute_arch`.
   - What's unclear: Does the roadmap author intend partial metrics removal in Phase 1 (e.g., only `whatif`, `dsm`, `testgap` panels which are GUI-only and MCP-only)?
   - Recommendation: Plan CLEN-04 as: remove `whatif_display.rs` panel and `metrics/whatif` module only (safe since `whatif` is only used by the MCP server and the GUI panel). Leave `grading`, `stability`, `arch`, `evo`, `rules`, `testgap`, and `dsm` for Phase 2. This gives a clean partial delivery.

2. **`sentrux-pro` private crate dependencies on MCP types**
   - What we know: `sentrux-core/src/app/mcp_server/` exports `McpState`, `ToolDef`, `ToolRegistry`, `run_mcp_server`; these are `pub` specifically for `sentrux-pro`
   - What's unclear: Whether `sentrux-pro` uses these and must be updated simultaneously
   - Recommendation: Flag this in the plan. If `sentrux-pro` cannot be updated, stub the MCP types as empty structs and mark them `#[deprecated]` rather than deleting.

3. **tree-sitter-javascript vs. tree-sitter-typescript for .tsx / .jsx**
   - What we know: `tree-sitter-typescript` covers `.ts` and `.tsx`; `tree-sitter-javascript` covers `.js`, `.jsx`
   - What's unclear: Whether the current `queries/javascript/tags.scm` works with both the TS grammar and the JS grammar for JSX
   - Recommendation: Test parse of a `.tsx` file with the TypeScript grammar and a `.jsx` file with the JavaScript grammar after adding the crates. This is a quick validation step.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | none (standard Cargo test runner) |
| Quick run command | `cargo test -p sentrux-core --lib 2>&1 \| tail -5` |
| Full suite command | `cargo test --workspace 2>&1 \| tail -20` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLEN-01 | Binary builds without MCP server; `sentrux mcp` no longer a valid subcommand | Build smoke test | `cargo build -p sentrux-bin 2>&1 \| grep -c error` (must be 0) | Wave 0 |
| CLEN-02 | No plugin loading at startup; no `~/.sentrux/plugins` access | Unit: `lang_registry::tests::test_registry_loads` still passes | `cargo test -p sentrux-core lang_registry` | ✅ exists |
| CLEN-03 | Rust/TS/JS files parse; other extensions return unknown lang | Unit: extend `lang_registry::tests` | `cargo test -p sentrux-core lang_registry` | ✅ partially |
| CLEN-04 | (narrow scope: `whatif` removed) Binary builds with no dead-code warnings | `cargo build -p sentrux-core 2>&1 \| grep -c "dead_code"` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo check --workspace`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -20`
- **Phase gate:** `cargo build --release --workspace` with zero warnings from removed subsystems before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `sentrux-core/src/analysis/lang_registry.rs` — existing `test_registry_loads` test will need updating once the static registry replaces the plugin-based one; add tests for `get_grammar_and_query("rust")`, `get_grammar_and_query("python")` (must return None), `detect_lang_from_ext("rs")` (must return "rust"), `detect_lang_from_ext("go")` (must return "unknown")
- [ ] No test for "silently skips unknown language files without error" — add an integration test that scans a directory containing `.py` and `.go` files and verifies no panic and no error in `ScanResult`

---

## Sources

### Primary (HIGH confidence)
- `sentrux-bin/src/main.rs` — direct inspection; all MCP, plugin, and auto-install entry points are at lines 44-46, 75-76, 79-116, 124, 132-135, 144-147, 148-150, 338-546, 672-737
- `sentrux-core/src/analysis/plugin/loader.rs` — direct inspection; `libloading` and `dirs` usage confirmed at lines 40, 167-192
- `sentrux-core/src/analysis/lang_registry.rs` — direct inspection; currently 100% plugin-based; all public API functions documented
- `sentrux-core/src/analysis/mod.rs` — direct inspection; `pub mod plugin` at line 15
- `sentrux-core/src/app/mod.rs` — direct inspection; `pub mod mcp_server` at line 11
- `sentrux-core/src/metrics/mod.rs` — direct inspection; `pub use evo as evolution` alias at line 28; used exclusively by `mcp_handlers_evo.rs`
- `.planning/codebase/CONCERNS.md` — `libloading` and memory leak pattern documented

### Secondary (MEDIUM confidence)
- tree-sitter Rust crate pattern (`.into()` from `LanguageFn`) — based on tree-sitter 0.24+ migration; requires `cargo check` verification against actual crate versions
- `sentrux-pro` concern re: MCP type dependencies — inferred from `pub` visibility on MCP types in `mcp_server/mod.rs:31`; not directly verified

### Tertiary (LOW confidence)
- CLEN-04 Phase 1 vs. Phase 2 boundary — analysis based on requirements text plus CLI command dependency graph; not confirmed by user

---

## Metadata

**Confidence breakdown:**
- CLEN-01 (MCP removal): HIGH — all touch points directly inspected in source
- CLEN-02 (plugin removal): HIGH — `plugin/` module fully inspected; dependency chain clear
- CLEN-03 (language narrowing): HIGH — `lang_registry.rs` fully inspected; static registry pattern well-understood
- CLEN-04 (metrics removal): MEDIUM — scope boundary is ambiguous per requirements text; CLI dependency confirmed

**Research date:** 2026-03-14
**Valid until:** 2026-04-14 (stable codebase; only external risk is tree-sitter crate version compatibility)
