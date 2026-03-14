# Sentrux — PMAT-Powered Code Visualization

## What This Is

Sentrux is a desktop code visualization tool that renders codebases as interactive treemaps with dependency edges, health overlays, and navigation. It is being refactored to use PMAT (paiml-mcp-agent-toolkit) as its analysis engine, replacing the current in-house metrics with PMAT's TDG grading, health scoring, and mutation testing. The tool targets developers working primarily in Rust, with secondary support for TypeScript (AWS CDK) and JavaScript (iframe widgets).

## Core Value

Show developers the health and evolution of their codebase through an interactive treemap powered by PMAT's analysis — past changes via git diff overlays, present state via TDG grades, and future direction via GSD phase overlays.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

- ✓ Squarified treemap visualization with pan/zoom — existing
- ✓ Dependency edge routing and rendering — existing
- ✓ Spatial indexing and hit testing — existing
- ✓ eframe/egui desktop GUI shell with panels and toolbar — existing
- ✓ Filesystem watching with live reload — existing
- ✓ Minimap navigation — existing
- ✓ Breadcrumb directory navigation — existing

### Active

<!-- Current scope. Building toward these. -->

- [ ] PMAT integrated as Rust library dependency for all code analysis
- [ ] TDG grades (A+ through F) displayed on treemap nodes
- [ ] PMAT health score visible in GUI panels
- [ ] PMAT mutation testing results accessible through GUI
- [ ] Simplified metrics — remove sentrux's own grading/rating system
- [ ] Language support narrowed to Rust, TypeScript, JavaScript only
- [ ] Git diff overlay — color-code treemap by changes in selectable time windows (15min, 1h, 1d, etc.)
- [ ] GSD phase overlay — color-code files by which GSD phase will touch them (future) or has touched them (past)
- [ ] Remove MCP server mode (PMAT provides this)
- [ ] Remove plugin system (no longer needed with narrowed language scope)

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- MCP server mode — PMAT already provides MCP integration for AI agents
- Plugin system for additional languages — narrowing to Rust/TS/JS only, PMAT handles analysis
- Multi-language support beyond Rust/TS/JS — user's projects don't need it
- Mobile or web deployment — desktop-only tool
- Own analysis engine — PMAT replaces all code analysis

## Context

- Sentrux is an existing Rust codebase (v0.3.12) with a Cargo workspace: `sentrux-core` (library) and `sentrux-bin` (binary)
- The current analysis layer (`sentrux-core/src/analysis/`) uses tree-sitter for parsing and custom metrics — this will be replaced by PMAT
- The visualization stack (treemap layout, edge routing, renderer) is mature and stays intact
- The GUI shell (eframe/egui, panels, toolbar, canvas) stays intact
- PMAT is at https://github.com/paiml/paiml-mcp-agent-toolkit — a Rust library with TDG grading, health scoring, mutation testing, and semantic search
- Small developer audience uses sentrux (open source)
- User's projects are primarily Rust, with TypeScript for AWS CDK and JavaScript for iframe widgets
- The shift is incremental: first milestone integrates PMAT, subsequent milestones add git/GSD overlays

## Constraints

- **Tech stack**: Must remain a native Rust desktop app using eframe/egui
- **Dependency**: PMAT must be usable as a Cargo library dependency (not CLI or MCP client)
- **Compatibility**: Treemap visualization, edge routing, and spatial indexing must remain functional throughout the refactor
- **Scope**: Rust, TypeScript, and JavaScript only — no other language support needed

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Use PMAT as Rust library dependency | Tight integration, no IPC overhead, same language | — Pending |
| Replace sentrux analysis with PMAT | PMAT is focused on AI development and Rust, avoids maintaining duplicate analysis | — Pending |
| Adopt PMAT's TDG grading model | Simpler than sentrux's current complex rating system, well-established A+-F scale | — Pending |
| Remove MCP server mode | PMAT already provides MCP server for AI agents | — Pending |
| Remove plugin system | Narrowing to 3 languages makes plugins unnecessary | — Pending |
| Incremental milestones | First integrate PMAT, then add overlays — reduces risk | — Pending |
| Git diff overlay (not timeline slider) | Shows changes on the existing treemap rather than requiring temporal navigation | — Pending |
| GSD phase overlay (not separate timeline) | Files color-coded by phase on treemap — consistent with diff overlay pattern | — Pending |

---
*Last updated: 2026-03-14 after initialization*
