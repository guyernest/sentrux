# Codebase Structure

**Analysis Date:** 2026-03-14

## Directory Layout

```
sentrux/                          # Cargo workspace root
├── Cargo.toml                    # Workspace manifest (members: sentrux-core, sentrux-bin)
├── Cargo.lock
├── install.sh                    # Install script for end users
├── sentrux-bin/                  # Thin binary crate (entry point only)
│   └── src/
│       └── main.rs               # CLI parsing, GPU probing, mode dispatch
├── sentrux-core/                 # All logic (library crate)
│   └── src/
│       ├── lib.rs                # Module declarations + public re-exports
│       ├── license.rs            # License tier detection
│       ├── analysis/             # Filesystem scan, parsing, graph extraction
│       │   ├── entry_points.rs   # Entry-point detection heuristics
│       │   ├── git.rs            # Git status integration
│       │   ├── lang_registry.rs  # Language → file extension mapping
│       │   ├── graph/            # Import/call/inherit graph construction
│       │   ├── parser/           # Tree-sitter batch parsing
│       │   │   ├── captures.rs   # Tree-sitter capture extraction
│       │   │   ├── imports.rs    # Import extraction from captures
│       │   │   ├── lang_extractors.rs  # Per-language structural extractors
│       │   │   └── strings.rs    # String literal handling
│       │   ├── plugin/           # Runtime tree-sitter plugin loader
│       │   │   ├── loader.rs     # Dynamic library loading
│       │   │   └── manifest.rs   # plugin.toml parsing + validation
│       │   ├── resolver/         # Import string → file path resolution
│       │   │   ├── helpers.rs    # Package index file detection, shared utils
│       │   │   ├── oxc.rs        # JS/TS resolution via oxc_resolver
│       │   │   └── suffix.rs     # Suffix-based path resolution
│       │   └── scanner/          # Full scan + incremental rescan
│       │       ├── common.rs     # ScanLimits, ScanResult, helpers
│       │       ├── rescan.rs     # Incremental rescan (patches snapshot)
│       │       └── tree.rs       # File → directory tree construction
│       ├── app/                  # egui UI, event loop, thread orchestration
│       │   ├── breadcrumb.rs     # Drill-down navigation breadcrumb
│       │   ├── canvas.rs         # Main canvas widget (hit testing, interaction)
│       │   ├── channels.rs       # Typed inter-thread message types
│       │   ├── draw_panels.rs    # Top-level panel layout and drawing
│       │   ├── prefs.rs          # User preferences persistence
│       │   ├── progress.rs       # Scan progress UI
│       │   ├── scan_threads.rs   # Scanner and layout worker thread spawning
│       │   ├── scanning.rs       # Scan initiation helpers
│       │   ├── settings_panel.rs # Settings drawer UI
│       │   ├── state.rs          # AppState (all mutable UI state, main thread only)
│       │   ├── status_bar.rs     # Bottom status bar
│       │   ├── toolbar.rs        # Top toolbar (open, mode controls)
│       │   ├── update_check.rs   # Background version check
│       │   ├── update_loop.rs    # egui update() — channel polling, frame dispatch
│       │   ├── watcher.rs        # Filesystem watcher setup
│       │   ├── mcp_server/       # Model Context Protocol stdio server
│       │   │   ├── handlers.rs   # Core tool handlers (scan, health, gate, arch)
│       │   │   ├── handlers_evo.rs  # Evolution/analysis tool handlers
│       │   │   ├── registry.rs   # ToolDef, ToolRegistry, dispatch
│       │   │   └── tools.rs      # build_registry() — all tool registrations
│       │   └── panels/           # Per-metric UI panels (right sidebar)
│       │       ├── activity_panel.rs
│       │       ├── arch_display.rs
│       │       ├── dsm_panel.rs
│       │       ├── evolution_display.rs
│       │       ├── health_display.rs
│       │       ├── metrics_panel.rs
│       │       ├── rules_display.rs
│       │       ├── testgap_display.rs
│       │       ├── ui_helpers.rs
│       │       └── whatif_display.rs
│       ├── core/                 # Shared types — no layer-specific logic
│       │   ├── heat.rs           # HeatTracker (per-file edit heat with decay)
│       │   ├── path_utils.rs     # Path manipulation utilities
│       │   ├── settings.rs       # Settings, Theme, ThemeConfig
│       │   ├── snapshot.rs       # Snapshot, FileEvent, ScanProgress
│       │   └── types.rs          # FileNode, StructuralAnalysis, FuncInfo, edges
│       ├── layout/               # File tree → positioned rects + routed edges
│       │   ├── aggregation.rs    # Edge path aggregation across all graph types
│       │   ├── blueprint.rs      # Blueprint grid layout
│       │   ├── blueprint_dag.rs  # Blueprint DAG layout variant
│       │   ├── routing.rs        # Edge routing algorithm
│       │   ├── spatial_index.rs  # Grid-based O(1) hit testing
│       │   ├── squarify.rs       # Squarified treemap algorithm
│       │   ├── treemap_layout.rs # Treemap layout orchestration
│       │   ├── types.rs          # RenderData, LayoutRectSlim, Anchor, etc.
│       │   ├── viewport.rs       # ViewportTransform (pan/zoom)
│       │   └── weight.rs         # File weight computation for sizing
│       ├── metrics/              # Health report computation from Snapshot
│       │   ├── grading.rs        # Letter grade computation
│       │   ├── stability.rs      # Coupling, entropy, cohesion metrics
│       │   ├── types.rs          # HealthReport, FileMetric, FuncMetric, etc.
│       │   ├── arch/             # Architecture metrics (Martin's metrics)
│       │   │   ├── distance.rs   # Distance from main sequence
│       │   │   └── graph.rs      # Architecture graph construction
│       │   ├── dsm/              # Design Structure Matrix
│       │   ├── evo/              # Git evolution metrics (churn, bus factor)
│       │   │   └── git_walker.rs # Git log walker via git2
│       │   ├── rules/            # .sentrux/rules.toml rule engine
│       │   │   └── checks.rs     # Rule evaluation against health/arch reports
│       │   ├── testgap/          # Test gap analysis
│       │   └── whatif/           # What-if impact simulation
│       ├── queries/              # Bundled tree-sitter query files (.scm)
│       │   ├── bash/tags.scm
│       │   ├── c/tags.scm
│       │   ├── cpp/tags.scm
│       │   ├── csharp/tags.scm
│       │   ├── css/tags.scm
│       │   ├── dockerfile/tags.scm
│       │   ├── elixir/tags.scm
│       │   ├── go/tags.scm
│       │   ├── haskell/tags.scm
│       │   ├── html/tags.scm
│       │   ├── java/tags.scm
│       │   ├── javascript/tags.scm
│       │   ├── kotlin/tags.scm
│       │   ├── lua/tags.scm
│       │   ├── ocaml/tags.scm
│       │   ├── php/tags.scm
│       │   ├── python/tags.scm
│       │   ├── r/tags.scm
│       │   ├── ruby/tags.scm
│       │   ├── rust/tags.scm
│       │   ├── scala/tags.scm
│       │   ├── scss/tags.scm
│       │   ├── swift/tags.scm
│       │   ├── typescript/tags.scm
│       │   └── zig/tags.scm
│       └── renderer/             # Functional rendering pipeline (egui)
│           ├── badges.rs         # Health/git status badge drawing
│           ├── colors.rs         # Color computation per color mode
│           ├── edge_routing.rs   # Per-frame edge path clipping
│           ├── edges.rs          # Dependency arrow drawing
│           ├── heat_overlay.rs   # Ripple animation overlay
│           ├── minimap.rs        # Navigation minimap
│           └── rects.rs          # File/directory block drawing
├── .sentrux/                     # Project config for sentrux-on-sentrux
│   └── rules.toml                # Architectural rules for this repo
├── .planning/                    # GSD planning documents
│   └── codebase/                 # Codebase analysis documents
├── .claude/                      # Claude project config
├── .claude-plugin/               # Claude plugin config
├── claude-plugin/                # Claude plugin skill definitions
│   └── skills/scan/
│       └── SKILL.md
├── assets/                       # Marketing/documentation assets
│   ├── logo.svg
│   └── screenshot*.png
└── .github/
    └── workflows/                # CI/CD GitHub Actions
```

## Directory Purposes

**`sentrux-bin/src/`:**
- Purpose: Thin binary entry point — CLI parsing, GPU backend probing, mode routing
- Contains: Single `main.rs` (~740 lines)
- Key files: `sentrux-bin/src/main.rs`

**`sentrux-core/src/analysis/`:**
- Purpose: Everything that transforms a directory on disk into a `Snapshot`
- Contains: Scanner, tree-sitter parser, import resolver, graph builder, plugin loader, git integration
- Key files: `sentrux-core/src/analysis/scanner/mod.rs` (main scan entry), `sentrux-core/src/analysis/parser/mod.rs`, `sentrux-core/src/analysis/graph/mod.rs`

**`sentrux-core/src/app/`:**
- Purpose: Application shell — egui UI components, thread management, MCP server
- Contains: `SentruxApp`, `AppState`, channels, worker threads, all UI panels
- Key files: `sentrux-core/src/app/mod.rs`, `sentrux-core/src/app/state.rs`, `sentrux-core/src/app/channels.rs`, `sentrux-core/src/app/update_loop.rs`

**`sentrux-core/src/core/`:**
- Purpose: Shared vocabulary types — no business logic, no layer affiliation
- Contains: `FileNode`, `Snapshot`, `Settings`, `HeatTracker`, error types
- Key files: `sentrux-core/src/core/types.rs`, `sentrux-core/src/core/snapshot.rs`, `sentrux-core/src/core/settings.rs`

**`sentrux-core/src/layout/`:**
- Purpose: Spatial computation — turns the file tree into positioned rectangles and edge paths
- Contains: Squarified treemap, blueprint DAG, edge routing, viewport transform, spatial index
- Key files: `sentrux-core/src/layout/mod.rs` (`compute_layout_from_snapshot`), `sentrux-core/src/layout/types.rs`

**`sentrux-core/src/metrics/`:**
- Purpose: Code quality analysis — coupling, complexity, cycles, architecture grading
- Contains: `compute_health`, arch metrics, DSM, git evolution, rule engine, test gap, what-if
- Key files: `sentrux-core/src/metrics/mod.rs` (`compute_health`), `sentrux-core/src/metrics/types.rs`, `sentrux-core/src/metrics/arch/mod.rs`

**`sentrux-core/src/queries/`:**
- Purpose: Tree-sitter `.scm` query files bundled into the binary via `include_str!`
- Contains: One `tags.scm` per language (25 languages built-in)
- Generated: No (hand-authored); Committed: Yes

**`sentrux-core/src/renderer/`:**
- Purpose: Pure rendering pipeline — given state, draw pixels; no mutation
- Contains: rect drawing, edge drawing, badge drawing, heat overlay, minimap
- Key files: `sentrux-core/src/renderer/mod.rs` (`render_frame`, `RenderContext`)

**`~/.sentrux/plugins/<lang>/`:**
- Purpose: User-installed language plugins (runtime, not in repo)
- Contains: `plugin.toml`, `grammars/<platform>.so`, `queries/tags.scm`
- Generated: Yes (downloaded by `sentrux plugin add-standard`); Committed: No

## Key File Locations

**Entry Points:**
- `sentrux-bin/src/main.rs`: `main()` — CLI dispatch, GUI launch
- `sentrux-core/src/app/mod.rs`: `SentruxApp` — top-level egui app struct
- `sentrux-core/src/app/mcp_server/mod.rs`: `run_mcp_server` — MCP stdio loop
- `sentrux-core/src/analysis/scanner/mod.rs`: `scan_directory` — core scan function

**Configuration:**
- `Cargo.toml`: Workspace manifest
- `sentrux-bin/Cargo.toml`: Binary crate deps
- `sentrux-core/Cargo.toml`: Library crate deps
- `.sentrux/rules.toml`: Architectural rules for this repository

**Core Logic:**
- `sentrux-core/src/core/types.rs`: `FileNode`, `StructuralAnalysis`, all edge types
- `sentrux-core/src/core/snapshot.rs`: `Snapshot` (central data type)
- `sentrux-core/src/app/state.rs`: `AppState` (all UI state)
- `sentrux-core/src/app/channels.rs`: `ScanMsg`, `ScanCommand`, `LayoutRequest`, `LayoutMsg`
- `sentrux-core/src/metrics/mod.rs`: `compute_health(snapshot)` — top-level metric entry
- `sentrux-core/src/layout/mod.rs`: `compute_layout_from_snapshot` — layout entry

**Testing:**
- `sentrux-core/src/analysis/graph/tests.rs`: Graph construction tests
- `sentrux-core/src/analysis/parser/tests.rs`, `tests2.rs`: Parser unit tests
- `sentrux-core/src/analysis/resolver/tests.rs`, `tests2.rs`: Resolver tests
- `sentrux-core/src/metrics/mod_tests.rs`, `mod_tests2.rs`: Metrics unit tests
- `sentrux-core/src/layout/tests.rs`, `tests2.rs`: Layout tests
- `sentrux-core/src/app/scanning_tests.rs`: Scanning integration tests

## Naming Conventions

**Files:**
- Module files: `snake_case.rs` (e.g., `lang_registry.rs`, `spatial_index.rs`)
- Sub-module directories: `snake_case/` with `mod.rs` inside
- Test files: co-located as `tests.rs` or `tests2.rs` within the module directory, or `mod_tests.rs` / `scanning_tests.rs` at the parent level
- Query files: all named `tags.scm`, one per language directory under `queries/`

**Directories:**
- Top-level modules: short, single-word names (`analysis`, `app`, `core`, `layout`, `metrics`, `renderer`)
- Sub-modules: descriptive snake_case (`mcp_server`, `lang_registry`, `blueprint_dag`)

**Types:**
- Structs: `PascalCase` (e.g., `FileNode`, `AppState`, `ScanCommand`, `HealthReport`)
- Enums: `PascalCase` with `PascalCase` variants (e.g., `ScanMsg::Complete`, `LayoutMode::Treemap`)
- Functions: `snake_case` (e.g., `compute_health`, `scan_directory`, `render_frame`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_FILES`, `FAN_OUT_THRESHOLD`)

## Where to Add New Code

**New CLI subcommand:**
- Add variant to `Command` enum in `sentrux-bin/src/main.rs`
- Add handler function `run_<name>` in `sentrux-bin/src/main.rs`
- Use `analysis::scanner::scan_directory` + `metrics::compute_health` as in existing commands

**New metric or code quality check:**
- Add computation in `sentrux-core/src/metrics/` (new flat module or sub-module under `metrics/`)
- Add result type in `sentrux-core/src/metrics/types.rs`
- Add field to `HealthReport` in `sentrux-core/src/metrics/types.rs`
- Populate it in `compute_health` in `sentrux-core/src/metrics/mod.rs`
- Store on `AppState` in `sentrux-core/src/app/state.rs`
- Add to `ScanReports` in `sentrux-core/src/app/channels.rs`

**New UI panel:**
- Create `sentrux-core/src/app/panels/<name>_display.rs`
- Register in `sentrux-core/src/app/panels/mod.rs`
- Add panel toggle bool to `AppState` in `sentrux-core/src/app/state.rs`
- Wire into `draw_panels.rs`

**New MCP tool:**
- Add `ToolDef` + handler in `sentrux-core/src/app/mcp_server/handlers.rs` or `handlers_evo.rs`
- Register in `sentrux-core/src/app/mcp_server/tools.rs` via `build_registry()`

**New built-in language (queries):**
- Create directory `sentrux-core/src/queries/<lang>/`
- Write `sentrux-core/src/queries/<lang>/tags.scm`
- Register in `sentrux-core/src/analysis/lang_registry.rs`
- Add `include_str!` loading in the parser module

**New language via plugin:**
- Use `sentrux plugin init <name>` to scaffold
- Plugin installs to `~/.sentrux/plugins/<name>/`
- Auto-loaded at startup; no core changes needed

**Utility/helper functions:**
- Shared path utilities: `sentrux-core/src/core/path_utils.rs`
- Analysis helpers: `sentrux-core/src/analysis/resolver/helpers.rs`
- Metrics test helpers: `sentrux-core/src/metrics/test_helpers.rs`

## Special Directories

**`.sentrux/`:**
- Purpose: Project-level sentrux configuration
- Contains: `rules.toml` (architectural rules enforced by `sentrux check`)
- Generated: No; Committed: Yes (intentional per-project config)

**`.planning/`:**
- Purpose: GSD planning documents for development workflow
- Contains: `codebase/` (analysis docs), phase plans
- Generated: By GSD tooling; Committed: Yes

**`assets/`:**
- Purpose: Marketing assets (logos, screenshots, demo files)
- Generated: No; Committed: Yes

**`~/.sentrux/plugins/`:**
- Purpose: Runtime language plugin directory (outside repo)
- Generated: Yes (auto-installed on first run via curl); Committed: No

---

*Structure analysis: 2026-03-14*
