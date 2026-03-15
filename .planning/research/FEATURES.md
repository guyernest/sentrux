# Feature Landscape

**Domain:** Code visualization + static analysis desktop tool (PMAT-powered, treemap + git overlays + GSD planning views)
**Researched:** 2026-03-14
**Note on sources:** WebSearch and Context7 were unavailable in this session. Analysis draws on training knowledge of peer tools (CodeScene, Sourcegraph, Understand, Structure101, Sonar, Embold, CodeCharta) and direct analysis of the existing sentrux codebase.

---

## Table Stakes

Features users expect when opening any code analysis visualization tool. Missing any of these and users assume the tool is broken or incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Treemap renders on open — no config required | First action is "open folder and see something" | Low | Already exists. Must survive PMAT integration without regression. |
| File nodes sized by LOC | Universal treemap convention — violating it breaks spatial intuition | Low | Already exists via tokei. PMAT must preserve line counts. |
| Color encodes one quality signal by default | Colorless treemap looks like a file tree; no signal = no value | Low | Currently health-grade color. Switching to TDG grade must be the default. |
| Grade label readable on node | Users need the letter grade on the block, not just color | Low | TDG A+ through F on nodes is Active requirement. Must be legible at normal zoom. |
| Pan and zoom | Large repos don't fit; zooming to a file is fundamental navigation | Low | Already exists. Stays intact. |
| Click a node → see file details | Treemap is useless without drilling into individual file metrics | Low | Already exists via panels. Must surface PMAT metrics (TDG, complexity, churn). |
| Directory grouping and collapse | Repo structure implies directory hierarchy in layout | Med | Already exists via squarified layout with directories. |
| Live reload on file save | Developers make a change and expect the view to update | Med | Already exists via `notify` watcher + rescan. |
| Scan progress indication | Large repos take seconds; a frozen window reads as a crash | Low | Already exists via status bar. Must remain accurate with PMAT scan path. |
| Breadcrumb navigation | Deep codebases need positional awareness | Low | Already exists. |
| Minimap | Spatial overview when zoomed in | Low | Already exists. |
| Dependency edges visible | The tool claims to show dependency graphs; edges missing = false advertising | Med | Already exists. Edge routing stays intact. |
| Clear indication of worst files | Users open the tool to find problems; the worst files must be obvious without hunting | Low | Implied by TDG grade coloring. Top-N worst list in panel is expected. |

---

## Differentiators

Features that distinguish sentrux from generic code analysis dashboards. Not expected on first open, but valuable once discovered and hard to find elsewhere in this combination.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| TDG grade on every treemap node | PMAT's TDG (Technical Debt Grade) is a recognized standard in AI-adjacent tooling; displaying it visually makes the invisible visible | Med | Core Active requirement. Depends on PMAT integration. |
| Git diff overlay — color by change recency | "What changed recently?" is the most common question when debugging a broken build or understanding unfamiliar code. Showing it on the treemap, not in a separate git log, is the differentiator. | Med | Active requirement. Time windows: 15min, 1h, 1d, 1w. Uses existing `git2`. |
| GSD phase overlay — color by planning phase | Connects the planning artifact (GSD phases) to the codebase. No other tool shows "these files are touched in Phase 3." Especially valuable for the target user (solo or small team following GSD). | High | Active requirement. Requires reading `.planning/` GSD files + mapping to file paths. |
| Past/present/future in one view | The combination of git overlay (past), TDG grade (present), and GSD overlay (future) on a single treemap is the core thesis. No competitor does this triple overlay. | High | Emerges from the combination of the three overlays — UI must allow switching between them cleanly. |
| PMAT health score panel | Beyond grade letters, PMAT's full health report (coupling, cycles, mutation score) in a dedicated panel gives expert users the numbers | Med | Active requirement. Panel surfacing PMAT's report. |
| PMAT mutation testing results | Mutation testing is rare in visualization tools; surfacing it alongside structural metrics ties test quality to code location | Med | Active requirement. PMAT provides this. |
| Hot-path detection via co-change coupling | Files that change together show up as temporal couples — "these two files always break together" before it causes an incident | Med | Already exists in `metrics/evo/`. PMAT may replace or augment this. |
| Overlay switcher — quick toggle between TDG / git-diff / GSD views | Single keyboard shortcut or toolbar button cycles through the three overlay modes. Competitors require separate tools/tabs. | Low | Implementation is a mode enum + re-color on switch. High impact for low effort. |
| Entry point detection with visual marker | Shows where the program starts, not just what files exist. Useful for unfamiliar codebases. | Low | Already exists in analysis layer. |
| Spatial memory — same layout across sessions | Treemap layout stays stable across re-scans of the same root, so users build spatial memory of problem areas | Low | eframe persistence already saves state. Layout determinism is already squarified. |

---

## Anti-Features

Features to explicitly NOT build. Each has been evaluated and rejected.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Own code analysis engine | Sentrux had one; it is being removed. Maintaining tree-sitter queries + custom metrics for 3+ languages is an unbounded maintenance tax. | Delegate all analysis to PMAT. Zero custom metric computation in sentrux. |
| MCP server mode | PMAT already provides the MCP server for AI agent use cases. Sentrux running a second MCP server is duplication and confusion. | Remove in this milestone. Users who want MCP point their agent at PMAT directly. |
| Language plugin system | Plugin system exists to support languages beyond Rust/TS/JS. That scope has been deliberately narrowed. | Remove along with MCP server removal. Hard-code Rust/TS/JS only via PMAT's analysis. |
| Multi-language support beyond Rust/TS/JS | User's projects don't need it. Adding more languages means maintaining PMAT analysis mappings for them. | PMAT already focuses on Rust/TS. Stick to that. |
| Web or mobile deployment | eframe/egui is a native desktop stack. Rewriting for web (wasm + egui) is a separate product. | Desktop only. Direct download binary. |
| Remote/team scan sharing | Syncing treemap state between developers requires a backend. This is a solo/small-team tool. | Out of scope permanently. Point at the repo on a shared filesystem if needed. |
| PR/branch comparison view | Diffing two branches in a treemap is compelling but requires a second scan + layout diff algorithm. High complexity, low immediate value for target user. | Use git diff overlay with a time window that covers the branch lifetime instead. |
| Interactive architecture editing | Letting users draw dependency rules on the treemap (drag-to-forbid) is a UX trap — complex to implement, rarely used. | Rules stay in `.sentrux/rules.toml`. CLI check/gate enforces them. |
| Inline code viewer | Clicking a file to see source inside the treemap duplicates an editor. Users already have an editor open. | On click, open file in system default editor via `open` / `xdg-open`. |
| CI badge / GitHub integration | Integrating with CI pipelines requires OAuth, webhooks, and a server. Out of scope for a desktop tool. | Use `sentrux gate` CLI command in CI. That's the integration point. |
| Historical treemap playback / timeline slider | A timeline scrubber that replays the codebase state over months is compelling but requires storing multiple snapshots. The git diff overlay achieves "show me change" without a full timeline. | Git diff overlay with configurable time windows. |
| Telemetry dashboard / usage analytics per file | Tracking which files developers click most adds user modeling complexity with no direct value. | Stick to code metrics, not user behavior metrics. |

---

## Feature Dependencies

```
PMAT library integration
  → TDG grade display on nodes          (no PMAT = no TDG grades)
  → PMAT health score panel             (no PMAT = no health report)
  → PMAT mutation testing panel         (no PMAT = no mutation results)
  → Simplified metrics (remove own grading)

git2 (already present)
  → Git diff overlay                    (git diff overlay consumes git2 history)
  → Co-change coupling view             (already exists, stays)

Git diff overlay
  → Time window selector UI             (overlay needs a time window control)

GSD phase overlay
  → GSD plan file reader                (must parse .planning/ files to know which phase touches which files)
  → File-to-phase mapping               (link GSD tasks/phases to file paths via patterns or explicit lists)
  → Overlay mode switcher               (user switches between TDG / git-diff / GSD overlays)

Overlay mode switcher
  → TDG grade display
  → Git diff overlay
  → GSD phase overlay
  (switcher requires all three overlay modes to exist before it is useful)

PMAT health score panel
  → File detail panel update            (existing panels must show PMAT data instead of sentrux data)
```

---

## MVP Recommendation

The milestone has a clear integration-first order. Do not build the git or GSD overlays until PMAT is stable.

**Priority 1 — Foundation (PMAT integration milestone):**
1. PMAT as Cargo dependency, analysis layer replaced — zero own metric computation
2. TDG grade visible on treemap nodes (color + label)
3. PMAT health score panel replacing current health panels
4. Mutation testing results accessible in file detail panel
5. MCP server mode removed; plugin system removed

**Priority 2 — Git overlay milestone:**
1. Git diff overlay with time window selector (15min, 1h, 1d, 1w, 30d)
2. Overlay mode switcher (toolbar toggle: TDG / git-diff)
3. Time window UI control in toolbar or sidebar

**Priority 3 — GSD overlay milestone:**
1. GSD plan file reader (parse `.planning/` directory structure)
2. File-to-phase mapping logic
3. GSD overlay on treemap (color by phase)
4. Extend overlay switcher to include GSD mode

**Defer indefinitely:**
- PR comparison view — git diff overlay with a wide time window approximates this
- Inline code viewer — OS default editor on double-click is sufficient
- Historical timeline playback — out of scope

---

## Competitive Context

Tools this competes with / differs from (training knowledge, MEDIUM confidence):

| Tool | How Sentrux Differs |
|------|---------------------|
| CodeScene | CodeScene does temporal coupling and hotspot analysis but is a cloud/server product. Sentrux is local, offline, instant. CodeScene has no GSD integration. |
| Sourcegraph | Code search + navigation, not visualization. No treemap, no health grades. |
| SonarQube / SonarCloud | Quality gates and issue tracking, not spatial visualization. No treemap. No git diff overlay on a canvas. |
| Structure101 | Architecture layering analysis and DSM, but no git diff overlay, no GSD integration, and not a native Rust binary. |
| CodeCharta | Treemap visualization similar to sentrux but focused on SonarQube metric import. No PMAT, no GSD. |
| Embold | Cloud-based code quality, no visualization. Discontinued. |

The unique combination that no competitor has: **PMAT TDG grades + git diff overlays + GSD planning overlays, all on the same interactive treemap, as a local native binary with zero cloud dependency.**

---

## Sources

- Direct analysis of sentrux codebase (2026-03-14): `.planning/PROJECT.md`, `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STACK.md`, `.planning/codebase/INTEGRATIONS.md`, `.planning/codebase/CONCERNS.md`
- Training knowledge of peer tools: CodeScene, Sourcegraph, SonarQube, Structure101, CodeCharta — MEDIUM confidence (no live verification in this session)
- PMAT capabilities from PROJECT.md description — MEDIUM confidence (PMAT repo not directly fetched)
