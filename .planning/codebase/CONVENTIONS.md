# Coding Conventions

**Analysis Date:** 2026-03-14

## Naming Patterns

**Files:**
- Snake_case for all Rust source files: `git_walker.rs`, `lang_extractors.rs`, `path_utils.rs`
- Test modules split into `tests.rs` and `tests2.rs` when the test count grows large
- Shared test helpers in dedicated `test_helpers.rs` files per module subtree
- `mod.rs` used for all directory modules: `metrics/arch/mod.rs`, `metrics/evo/mod.rs`

**Functions:**
- Snake_case for all functions: `compute_health`, `detect_god_files`, `build_adjacency_list`
- Predicate functions prefixed with `is_`: `is_test_file`, `is_same_module`, `is_mod_declaration_edge`, `is_dominant_dir`, `is_called`
- Builder functions prefixed with `build_`: `build_body_hash_map`, `build_call_target_set`, `build_adjacency_list`
- Collector functions prefixed with `collect_`: `collect_dead_functions`, `collect_file_calls`, `collect_duplicate_groups`
- Compute functions prefixed with `compute_`: `compute_health`, `compute_fan_maps`, `compute_coupling_score`
- Detect functions prefixed with `detect_`: `detect_god_files`, `detect_cycles`, `detect_entry_points`

**Variables:**
- Snake_case throughout: `fan_out`, `import_edges`, `total_funcs`, `code_file_count`
- Short abbreviations used in tight data structs (serialized JSON): `n` (name), `sl` (start line), `el` (end line), `ln` (line count), `cc` (cyclomatic complexity), `cog` (cognitive complexity), `pc` (param count), `bh` (body hash), `co` (call-outs), `sa` (structural analysis)

**Types / Structs / Enums:**
- PascalCase: `FileNode`, `StructuralAnalysis`, `FuncInfo`, `HealthReport`, `TarjanState`, `GraphResult`
- Trait names PascalCase: `GraphBuilder`, `EvolutionAnalyzer`

**Constants:**
- SCREAMING_SNAKE_CASE with literature references: `CC_THRESHOLD_HIGH`, `FUNC_LENGTH_THRESHOLD`, `FAN_OUT_THRESHOLD`
- All magic numbers are named constants, never inline literals in logic

## Module Structure

**Visibility:**
- `pub` for cross-crate API surface (types and functions used by `sentrux-bin` or `sentrux-pro`)
- `pub(crate)` for internal helpers shared across modules: `pub(crate) fn is_package_index_file`, `pub(crate) const CC_THRESHOLD_HIGH`
- `pub(crate)` for internal structs: `pub(crate) struct FileMetrics`
- Private (`fn`) for module-internal helpers

**Module files:**
- Every directory module has `mod.rs` as the entry point
- Sub-module imports inside `mod.rs` documented with inline comments showing file relationships: `pub mod arch; // arch/mod.rs + graph.rs + distance.rs`
- Re-exports for backward compatibility explicitly noted: `pub use evo as evolution;`

## Documentation Comments

**File-level:**
- Every `mod.rs` starts with a `//!` module-level doc comment explaining:
  1. What the module does (one-line summary)
  2. Key public functions or types
  3. References to academic literature when applicable
- Example pattern from `sentrux-core/src/metrics/mod.rs`:
  ```rust
  //! Code health metrics (Constantine & Yourdon 1979, McCabe 1976, Martin).
  //!
  //! Top-level module that orchestrates all metric computations...
  //! Key function: `compute_health` produces a `HealthReport` from a `Snapshot`.
  ```

**Function-level:**
- All public and `pub(crate)` functions have `///` doc comments
- Comments explain WHY (design rationale, literature source) not just WHAT
- Bug fixes documented inline with "BUG FIX:" prefix and explanation
- Example:
  ```rust
  /// Check if two files belong to the same module.
  /// BUG FIX: removed asymmetric root-level-file exception. Previously,
  /// `src/app.rs` (module "src") was treated as intra-module with ALL subdirs...
  ```

**Inline comments:**
- Section dividers use `// ── Section Name ──` ASCII art pattern
- `[ref:XXXXXXXX]` tags used to cross-reference design decisions across files:
  ```rust
  // No arbitrary weights — overall = floor(mean). [ref:736ae249]
  pub(crate) const CC_THRESHOLD_HIGH: u32 = 15; // ── Named constants — no magic numbers [ref:736ae249] ──
  ```

## Import Organization

**Order in source files:**
1. `use super::` (parent module items)
2. `use crate::` (absolute crate paths)
3. `use std::` (standard library)

**Path style:**
- Absolute crate paths preferred over relative for cross-module imports: `use crate::core::types::FileNode`
- `use super::*` only in test files that are tightly coupled to the parent module

**No path aliases** — the project does not use `use X as Y` aliases except for the backward-compat re-export: `pub use evo as evolution`

## Error Handling

**Strategy:** Fallible public APIs return `Result<T, String>` (not `thiserror` Error types):
```rust
pub(crate) fn walk_git_log(root: &Path, lookback_days: u32) -> Result<Vec<CommitRecord>, String>
fn compute_evo_report(...) -> Result<EvolutionReport, String>
```

**Patterns:**
- `unwrap()` used only when the invariant is guaranteed by prior logic (e.g. stack manipulation in Tarjan's algorithm), never on external input
- `expect()` used only in test helpers where panics are acceptable
- `match` preferred over `?` for complex error paths that need logging or skip logic
- Silent skips with counter logging for non-critical errors during git log walking:
  ```rust
  Err(_) => { skips.oid += 1; continue; }
  ```
- Internal computation functions return bare values (not `Result`) and handle missing data via `Option` patterns with `unwrap_or`, `filter_map`, `and_then`

**Option patterns:**
- `Option<T>` used throughout for optional fields on data structs (`sa`, `children`, `cc`, `cog`)
- `filter_map` preferred for collection transformations over explicit `match` loops
- Serde attributes control JSON serialization: `#[serde(skip_serializing_if = "Option::is_none")]`

## Code Style

**Formatting:**
- Standard `rustfmt` formatting implied (no custom `.rustfmt.toml` found)
- Trailing commas in struct literals and match arms
- Closures on same line for short lambdas, multi-line for complex ones

**Linting:**
- `#[allow(dead_code)]` used sparingly with justification comment:
  ```rust
  #[allow(dead_code)] // Used by tests and legacy callers
  pub(crate) fn grade_entropy(v: f64) -> char {
  ```

**Function design:**
- Small, single-purpose functions — each named after exactly what it does
- Helper functions extracted for repeated patterns: `collect_functions_exceeding` abstracts over threshold checks
- Struct fields initialized via struct literal (not builder pattern) for data structs

**Line length:**
- Long chained iterator calls are wrapped at method boundaries
- Struct initializations with many fields formatted one-field-per-line
- Inline `if-else` on single line for simple grade checks: `if v <= 0.20 { 'A' } else if v <= 0.35 { 'B' }...`

---

*Convention analysis: 2026-03-14*
