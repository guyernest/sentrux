# Testing Patterns

**Analysis Date:** 2026-03-14

## Test Framework

**Runner:**
- Rust built-in `cargo test`
- No external test framework — standard `#[test]` attributes throughout
- Config: no `jest.config.*` or `vitest.config.*` — pure Rust `cargo test`

**Assertion Library:**
- Standard `assert_eq!`, `assert_ne!`, `assert!` macros
- Floating-point comparisons use explicit epsilon: `assert!((result - 0.5).abs() < f64::EPSILON)`
- Message context provided to assertions: `assert_eq!(x, y, "explanation of what should hold")`

**Run Commands:**
```bash
cargo test                   # Run all tests
cargo test --package sentrux-core  # Run only core library tests
cargo test metrics           # Run tests matching "metrics" in name
cargo test -- --nocapture    # Show println! output
```

## Test File Organization

**Location:**
- Tests are separated into dedicated files, NOT inline `#[cfg(test)] mod tests { }` blocks within source files (except for small unit tests on utility functions)
- Large test suites split across `tests.rs` and `tests2.rs` per module: `metrics/mod_tests.rs`, `metrics/mod_tests2.rs`, `analysis/graph/tests.rs`, `analysis/graph/tests2.rs`

**Naming:**
- Test files: `tests.rs`, `tests2.rs` (sequential numbering when split)
- Test helper files: `test_helpers.rs` — one per module subtree
- Test functions: `snake_case` describing the property being tested: `empty_graph_is_healthy`, `detects_simple_cycle`, `coupling_score_correct`

**Registration in `mod.rs`:**
```rust
#[cfg(test)]
pub(crate) mod test_helpers;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod mod_tests2;
```

**Structure:**
```
sentrux-core/src/
├── metrics/
│   ├── mod.rs                  # #[cfg(test)] mod mod_tests; mod mod_tests2;
│   ├── mod_tests.rs            # Main metrics integration tests
│   ├── mod_tests2.rs           # Additional metrics tests (overflow from tests.rs)
│   ├── test_helpers.rs         # Shared: edge(), file(), snap_with_edges()
│   ├── arch/
│   │   ├── mod.rs              # #[cfg(test)] mod tests; mod tests2;
│   │   ├── tests.rs
│   │   └── tests2.rs
│   ├── dsm/
│   │   ├── mod.rs
│   │   └── tests.rs
│   ├── evo/
│   │   ├── mod.rs
│   │   └── tests.rs
│   ├── rules/
│   │   ├── mod.rs
│   │   └── tests.rs
│   ├── testgap/
│   │   ├── mod.rs
│   │   └── tests.rs
│   └── whatif/
│       ├── mod.rs
│       └── tests.rs
├── analysis/
│   ├── mod.rs                  # #[cfg(test)] pub(crate) mod test_helpers;
│   ├── test_helpers.rs         # make_file() for analysis tests
│   ├── graph/
│   │   ├── mod.rs
│   │   ├── tests.rs
│   │   └── tests2.rs
│   └── parser/
│       ├── mod.rs
│       ├── tests.rs
│       └── tests2.rs
├── core/
│   └── path_utils.rs           # Inline #[cfg(test)] mod tests {} for unit tests
└── layout/
    ├── mod.rs
    ├── test_helpers.rs         # make_file(), make_dir(), simple_snapshot(), run_layout()
    ├── tests.rs
    └── tests2.rs
```

## Test Structure

**Suite Organization:**
Tests are organized into logical groups using inline comments with `// ── Group Name ──` ASCII headers:
```rust
// ── Boundary test: empty graph → grade A, no issues ──
#[test]
fn empty_graph_is_healthy() { ... }

// ── Symmetry test: A→B and B→A form a cycle ──
#[test]
fn detects_simple_cycle() { ... }
```

**Test module wrapper (for tests that need full `use` imports):**
```rust
#[cfg(test)]
mod tests {
    use crate::metrics::*;
    use crate::metrics::test_helpers::{edge, file, snap_with_edges};

    #[test]
    fn test_name() { ... }
}
```

**Top-level tests (no wrapper needed when `use super::*` suffices):**
```rust
use super::*;
use crate::metrics::test_helpers::edge;

#[test]
fn empty_dsm() {
    let dsm = build_dsm(&[]);
    assert_eq!(dsm.size, 0);
}
```

**File-level doc comment on every test file:**
```rust
//! Tests for architectural rule enforcement (`metrics::rules`).
//!
//! Validates rule checking against snapshots: forbidden dependency detection,
//! layer violation checks, and rule pass/fail logic. Tests cover boundary
//! (no rules = all pass), oracle (known violations produce known failures),
//! and conservation (adding a rule never removes existing violations).
//! Uses synthetic snapshots with controlled import edges.
```

## Mocking

**Framework:** No external mocking library. The codebase uses trait-based abstractions for testability.

**Trait-based injection:**
```rust
// Production trait defined in module
pub trait GraphBuilder {
    fn build(&self, files: &[&FileNode], scan_root: Option<&Path>, max_call_targets: usize) -> GraphResult;
}

// Evolution analyzer trait (metrics/evo/mod.rs)
pub trait EvolutionAnalyzer {
    fn analyze(...) -> Result<EvolutionReport, String>;
    fn compute_churn(...) -> Result<HashMap<String, FileChurn>, String>;
}
```

**What to mock:**
- External I/O: filesystem access and git repository operations use `tempdir` pattern with actual filesystem
- Git operations tested via temp directories created inline per test

**What NOT to mock:**
- Pure computation functions — tested directly with synthetic input data
- Tree-sitter parser — oracle tests feed raw source bytes and assert exact output

## Fixtures and Factories

**Shared test helpers are always in dedicated `test_helpers.rs` files**, never duplicated across test modules.

**`metrics/test_helpers.rs` — core fixture builders:**
```rust
/// Build a minimal ImportEdge from two path strings.
pub fn edge(from: &str, to: &str) -> ImportEdge { ... }

/// Build a minimal FileNode (non-dir, 100 lines, rust lang) from a path.
pub fn file(path: &str) -> FileNode { ... }

/// Build a minimal snapshot with given import edges and files.
pub fn snap_with_edges(edges: Vec<ImportEdge>, files: Vec<FileNode>) -> Snapshot { ... }
```

**`analysis/test_helpers.rs` — analysis-layer fixture builders:**
```rust
/// Build a minimal FileNode for testing graph/resolver logic.
pub fn make_file(name: &str, path: &str, lang: &str, sa: Option<StructuralAnalysis>) -> FileNode { ... }
```

**`layout/test_helpers.rs` — layout fixture builders:**
```rust
pub fn default_focus() -> FocusMode { FocusMode::All }
pub fn empty_entry_points() -> HashSet<String> { HashSet::new() }
pub fn no_hidden() -> HashSet<String> { HashSet::new() }
pub fn make_file(name: &str, path: &str, lines: u32) -> FileNode { ... }
pub fn make_dir(name: &str, path: &str, children: Vec<FileNode>) -> FileNode { ... }
pub fn simple_snapshot() -> Snapshot { ... }  // canonical multi-file tree for layout tests
pub fn run_layout(snap, size_mode, scale_mode, layout_mode, vw, vh) -> RenderData { ... }
```

**Local helpers** are defined inside test modules for module-specific needs not shared elsewhere:
```rust
fn entry(file: &str) -> EntryPoint {
    EntryPoint { file: file.to_string(), func: "main".to_string(), lang: "rust".to_string(), confidence: "high".to_string() }
}
```

## Coverage

**Requirements:** None formally enforced (no coverage threshold in CI config found).

**View Coverage:**
```bash
cargo test                    # No coverage tooling configured in repo
# For manual coverage: cargo tarpaulin or cargo llvm-cov
```

## Test Types

**Unit Tests:**
- Scope: single pure function with synthetic inputs
- Examples: `grade_bus_factor_boundary`, `empty_dsm`, `glob_star_matches_direct_children`, `depth_2_grouping`
- Located inline in `path_utils.rs` for small utilities, or in dedicated `tests.rs` files

**Integration Tests:**
- Scope: full pipeline from `ImportEdge`/`FileNode` input through `compute_health`, `build_graphs`, `check_rules`, `build_dsm`
- Always use the shared fixture builders (`snap_with_edges`, `edge`, `file`)
- Examples: `detects_simple_cycle`, `constraint_max_cycles_catches_violations`, `layer_violation_detected`

**Parser Oracle Tests** (`analysis/parser/tests.rs`):
- Feed raw source code bytes in a specific language to `parse_bytes`
- Assert exact function counts, class counts, import counts
- Purpose: regression guards for tree-sitter extraction logic
- Example:
  ```rust
  #[test]
  fn oracle_python() {
      let code = br#"...actual Python source..."#;
      let sa = parse_bytes(code, "python").expect("python parse failed");
      assert_eq!(sa.functions.as_ref().unwrap().len(), 5);
  }
  ```

**E2E Tests:** Not used. No integration test crate or separate `tests/` directory at crate root.

## Common Patterns

**Property names in test comments:**
Tests are categorized by testing property type in comments:
- `// ── Boundary test: empty graph → grade A ──`
- `// ── Oracle test: known input produces known output ──`
- `// ── Invariance test: X doesn't change Y ──`
- `// ── Symmetry test: A→B and B→A ──`
- `// ── Monotonicity test: more X = worse Y ──`
- `// ── Conservation test: edge count preserved ──`
- `// ── Idempotency test: same result when run twice ──`

**Floating-point assertions:**
```rust
assert!((report.coupling_score - 0.5).abs() < f64::EPSILON);
// Or with message:
assert!((report.coupling_score - 0.5).abs() < f64::EPSILON,
    "1 cross-module out of 2 edges = 50% coupling");
```

**Filesystem-dependent tests use temp dirs with cleanup:**
```rust
#[test]
fn call_edges_require_import() {
    let tmp = std::env::temp_dir().join("sentrux_test_call_import_gate");
    let _ = std::fs::remove_dir_all(&tmp);   // cleanup before
    std::fs::create_dir_all(&tmp).unwrap();

    // ... test logic using tmp as scan_root ...

    let _ = std::fs::remove_dir_all(&tmp);   // cleanup after
}
```

**TOML parsing tests verify deserialization directly:**
```rust
#[test]
fn parse_minimal_rules() {
    let toml = r#"
[constraints]
max_cycles = 0
"#;
    let config: RulesConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.constraints.max_cycles, Some(0));
}
```

---

*Testing analysis: 2026-03-14*
