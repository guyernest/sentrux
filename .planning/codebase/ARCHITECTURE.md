# Architecture

**Analysis Date:** 2026-03-14

## Pattern Overview

**Overall:** Layered monolith with three runtime modes (GUI, CLI, MCP), split across two Rust crates in a Cargo workspace.

**Key Characteristics:**
- `sentrux-bin` is a thin entry-point crate; all logic lives in `sentrux-core`
- Worker threads (scanner, layout) communicate with the UI thread exclusively via typed crossbeam channels — no shared mutable state
- `Snapshot` is the central immutable data type passed across all layers via `Arc<Snapshot>`; it is never mutated after construction
- The renderer is purely functional: given a `RenderContext` it draws — it never imports `AppState`
- Three runtime modes share the same analysis engine: GUI (`eframe`/`egui`), CLI (`sentrux check`, `sentrux gate`), and MCP stdio server (`sentrux mcp`)

## Layers

**Binary Layer (`sentrux-bin`):**
- Purpose: CLI parsing, GPU backend probing, dispatch to core
- Location: `sentrux-bin/src/main.rs`
- Contains: `Cli`, `Command`, `PluginAction` structs; `run_gui`, `run_check`, `run_gate`, `run_plugin`, `auto_install_plugins_if_needed`
- Depends on: `sentrux-core` (all logic), `clap`, `eframe`
- Used by: End user

**Core Layer (`sentrux-core`):**
- Purpose: All analysis, metrics, visualization, MCP server
- Location: `sentrux-core/src/lib.rs`
- Contains: six top-level modules: `analysis`, `app`, `core`, `layout`, `metrics`, `renderer`
- Depends on: `eframe`, `egui`, `tree-sitter`, `tokei`, `rayon`, `crossbeam-channel`, `git2`, `oxc_resolver`, `notify`, `serde`
- Used by: `sentrux-bin`, and optionally `sentrux-pro` (private crate)

**Core Types (`sentrux-core/src/core`):**
- Purpose: Canonical shared data types — no layer-specific logic
- Location: `sentrux-core/src/core/`
- Contains: `FileNode`, `Snapshot`, `StructuralAnalysis`, `FuncInfo`, `ImportEdge`, `CallEdge`, `InheritEdge`, `EntryPoint`, `Settings`, `HeatTracker`, `AppError`
- Depends on: serde, egui (for Pos2 in settings)
- Used by: every other module

**Analysis Layer (`sentrux-core/src/analysis`):**
- Purpose: Walk filesystem, count lines, parse structure via tree-sitter, build dependency graphs
- Location: `sentrux-core/src/analysis/`
- Contains: `scanner` (full scan + rescan), `parser` (tree-sitter batch parsing), `graph` (builds import/call/inherit edges), `resolver` (resolves import strings to file paths), `lang_registry` (language → extension mapping), `plugin` (runtime plugin loader), `git` (git status), `entry_points`
- Depends on: `core`, `queries/` (.scm files bundled as include_str!), `tokei`, `rayon`, `ignore`, `tree-sitter`, `oxc_resolver`
- Used by: `app` (scanner thread), `mcp_server` (direct scan calls), CLI modes

**Metrics Layer (`sentrux-core/src/metrics`):**
- Purpose: Compute code health reports from a `Snapshot`
- Location: `sentrux-core/src/metrics/`
- Contains: `compute_health` (main entry), `arch` (abstractness/instability/distance), `dsm` (Design Structure Matrix), `evo` (git evolution: churn, bus factor, temporal coupling), `testgap` (untested file detection), `rules` (rule engine against `.sentrux/rules.toml`), `stability`, `grading`, `whatif`
- Depends on: `core` (Snapshot, FileNode, ImportEdge), `git2` (in `evo`)
- Used by: `app` (scanner thread after scan completes), `mcp_server`, CLI modes

**Layout Layer (`sentrux-core/src/layout`):**
- Purpose: Transform file tree + metrics into positioned rectangles and routed edges
- Location: `sentrux-core/src/layout/`
- Contains: `treemap_layout` (squarified treemap), `blueprint`/`blueprint_dag` (grid DAG layout), `routing` (edge path routing), `aggregation` (edge path aggregation), `spatial_index` (O(1) hit testing), `viewport` (pan/zoom transform), `weight`, `types`
- Depends on: `core` (Snapshot, FileNode, Settings), `rayon` (parallel edge routing)
- Used by: `app` (layout thread), `mcp_server`

**Renderer (`sentrux-core/src/renderer`):**
- Purpose: Draw the visualization frame onto an egui Painter
- Location: `sentrux-core/src/renderer/`
- Contains: `rects` (file/directory blocks), `edges` (dependency arrows), `badges` (health/git indicators), `heat_overlay` (live edit heat), `minimap`, `colors`, `edge_routing`
- Depends on: `core`, `layout`, `metrics::arch`, `egui`
- Used by: `app/canvas.rs`

**App Layer (`sentrux-core/src/app`):**
- Purpose: egui UI, event loop, thread orchestration
- Location: `sentrux-core/src/app/`
- Contains: `SentruxApp` (top-level eframe app), `AppState` (all mutable UI state), `channels` (typed inter-thread messages), `scan_threads` (scanner worker), `update_loop` (egui frame dispatch), `panels/` (UI panels per metric), `mcp_server/` (stdio MCP server), `watcher` (filesystem watch), `toolbar`, `canvas`, `settings_panel`, `breadcrumb`, `status_bar`, `prefs`, `update_check`
- Depends on: all other modules
- Used by: `sentrux-bin` (run_gui instantiates SentruxApp)

**MCP Server (`sentrux-core/src/app/mcp_server`):**
- Purpose: Model Context Protocol stdio server for AI agent integration
- Location: `sentrux-core/src/app/mcp_server/`
- Contains: `registry` (ToolDef, ToolRegistry, dispatch, license gating), `handlers` (scan/health/gate/arch tools), `handlers_evo` (evolution analysis tools), `tools` (build_registry registration point)
- Depends on: `analysis`, `metrics`, `license`
- Used by: `run_mcp_server` called from `sentrux-bin` or `sentrux-pro`

## Data Flow

**GUI Scan Flow:**

1. User opens folder → `app/toolbar.rs` sets `state.folder_picker_requested`
2. Background file picker dialog sends path via `folder_picker_rx`
3. `scan_threads.rs` sends `ScanCommand::FullScan` → scanner thread
4. Scanner thread: `analysis::scanner::scan_directory` walks files, counts lines (tokei), gets git status, parses (tree-sitter), builds graphs
5. Scanner emits `ScanMsg::TreeReady` (partial snapshot, enables early rendering)
6. Scanner emits `ScanMsg::Complete` with full `Arc<Snapshot>` + `ScanReports`
7. `update_loop.rs` receives `ScanMsg::Complete` → stores on `AppState`, sends `LayoutRequest` to layout thread
8. Layout thread computes `RenderData` (rects + edge paths) → sends `LayoutMsg::Ready`
9. `update_loop.rs` stores `RenderData` on `AppState`
10. `canvas.rs` draws frame: calls `renderer::render_frame` with a `RenderContext` built from `AppState`

**Filesystem Watch Flow:**

1. `watcher.rs` sets up `notify` watcher for scanned root
2. Changed paths accumulate in `state.pending_changes`
3. After debounce, `scan_threads.rs` sends `ScanCommand::Rescan` with changed paths + old snapshot
4. `analysis::scanner::rescan` patches only changed files in the existing snapshot

**CLI Check/Gate Flow:**

1. `run_check(path)` / `run_gate(path)` called directly from `main`
2. Calls `analysis::scanner::scan_directory` synchronously
3. Calls `metrics::compute_health` and `metrics::arch::compute_arch`
4. CLI check: `metrics::rules::check_rules` against `.sentrux/rules.toml`
5. Gate: `metrics::arch::ArchBaseline::diff` against `.sentrux/baseline.json`
6. Exit code 0 (pass) or 1 (violations)

**MCP Flow:**

1. `run_mcp_server` reads JSON-RPC from stdin line by line
2. Routes tool name to handler via `ToolRegistry::dispatch`
3. Handlers call `analysis::scanner::scan_directory` and metrics functions
4. `McpState` caches `Snapshot`, `HealthReport`, `ArchReport` across requests
5. Results returned as JSON-RPC response to stdout

**State Management:**
- `AppState` is owned exclusively by the main (egui) thread
- Worker threads never access `AppState` directly
- Typed channels (`ScanMsg`, `LayoutMsg`, `ScanCommand`, `LayoutRequest`) carry all inter-thread data
- Generation counters on scan/layout messages allow stale-result rejection after rapid directory switches
- `Arc<Snapshot>` is shared across threads without cloning data

## Key Abstractions

**`Snapshot`:**
- Purpose: Immutable complete scan result — file tree + all three dependency graphs + entry points
- Examples: `sentrux-core/src/core/snapshot.rs`
- Pattern: Constructed once per scan; passed as `Arc<Snapshot>` to layout, metrics, renderer

**`FileNode`:**
- Purpose: Node in scanned file tree (file or directory)
- Examples: `sentrux-core/src/core/types.rs`
- Pattern: Contains line counts, git status, language, and optional `StructuralAnalysis`; children present only for directories

**`RenderData`:**
- Purpose: Pre-computed layout output ready for GPU rendering
- Examples: `sentrux-core/src/layout/types.rs`
- Pattern: Flat list of `LayoutRectSlim` + edge paths + anchor map; computed on layout thread, consumed by renderer

**`HealthReport`:**
- Purpose: Aggregated code health metrics from a snapshot
- Examples: `sentrux-core/src/metrics/types.rs`
- Pattern: Produced by `metrics::compute_health(snapshot)`; contains coupling, cycles, god files, complexity, duplication, grade

**`ToolDef` / `ToolRegistry`:**
- Purpose: MCP tool registration, dispatch, and license gating
- Examples: `sentrux-core/src/app/mcp_server/registry.rs`
- Pattern: Each tool is a `ToolDef` with a JSON schema + handler fn; registry dispatches by name and enforces tier requirements

**Language Plugins:**
- Purpose: Runtime-loaded tree-sitter grammars for additional languages
- Examples: `sentrux-core/src/analysis/plugin/`
- Pattern: Plugins in `~/.sentrux/plugins/<lang>/` with `plugin.toml` manifest + compiled `.so`/`.dylib` + `queries/tags.scm`

## Entry Points

**GUI (`run_gui`):**
- Location: `sentrux-bin/src/main.rs:579`
- Triggers: Default CLI invocation or `sentrux scan <path>`
- Responsibilities: Probe GPU backends, construct `SentruxApp`, start eframe event loop

**CLI Check (`run_check`):**
- Location: `sentrux-bin/src/main.rs:166`
- Triggers: `sentrux check [path]`
- Responsibilities: Synchronous scan, rules evaluation, stdout report, non-zero exit on violations

**CLI Gate (`run_gate`):**
- Location: `sentrux-bin/src/main.rs:235`
- Triggers: `sentrux gate [--save] [path]`
- Responsibilities: Synchronous scan, baseline comparison or save, non-zero exit on regression

**MCP Server (`run_mcp_server`):**
- Location: `sentrux-core/src/app/mcp_server/mod.rs:43`
- Triggers: `sentrux mcp` or `sentrux --mcp`
- Responsibilities: stdin JSON-RPC read loop, tool dispatch, cached scan state across calls

**Plugin Manager (`run_plugin`):**
- Location: `sentrux-bin/src/main.rs:338`
- Triggers: `sentrux plugin <list|add|add-standard|remove|init|validate>`
- Responsibilities: Plugin directory management, download from GitHub releases, validation

## Error Handling

**Strategy:** `Result<T, AppError>` for recoverable errors; `eprintln!` for scan diagnostics; panic with `catch_unwind` only at GPU backend boundary.

**Patterns:**
- `AppError` enum defined in `sentrux-core/src/core/types.rs` with `thiserror`
- Scanner errors are sent as `ScanMsg::Error(String, u64)` to the UI thread (never panic)
- GPU backend failures in `run_gui` use `std::panic::catch_unwind` to try fallback backends before giving up
- MCP handlers return JSON error objects for tool failures; never abort the server loop
- CLI check/gate return integer exit codes (0 = pass, 1 = failure/violations)

## Cross-Cutting Concerns

**Logging:** `eprintln!` with structured prefixes (e.g. `[scan]`, `[gpu]`, `[state]`); no logging framework
**Validation:** Input validation at scanner entry (`root.is_dir()` check); plugin validation via `PluginManifest::validate_query_captures`
**Authentication/Licensing:** `sentrux-core/src/license.rs` exposes `current_tier() -> Tier`; MCP tool registry gates pro tools against tier; `sentrux-pro` private crate adds pro tool handlers via `register_extra` callback

---

*Architecture analysis: 2026-03-14*
