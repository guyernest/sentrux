# Technology Stack

**Analysis Date:** 2026-03-14

## Languages

**Primary:**
- Rust 1.94.0 - All application code in `sentrux-core/` and `sentrux-bin/`

**Secondary:**
- Shell (sh) - Install script `install.sh`
- TOML - Configuration files (`.sentrux/rules.toml`, plugin manifests)
- Tree-sitter query language (`.scm`) - Language grammar queries in `sentrux-core/src/queries/`

## Runtime

**Environment:**
- Native binary — no runtime, compiled to a single executable

**Package Manager:**
- Cargo 1.94.0
- Lockfile: `Cargo.lock` present and committed

## Workspace Structure

This is a Cargo workspace with two crates:
- `sentrux-core` (`sentrux-core/Cargo.toml`) — library crate, version 0.3.12, all logic
- `sentrux-bin` (`sentrux-bin/Cargo.toml`) — binary crate `sentrux`, version 0.3.12, entry point only

Workspace manifest: `Cargo.toml` (root)

Shared workspace dependencies:
- `eframe = "0.31"` with features `["wgpu", "persistence"]`
- `egui = "0.31"`

## Frameworks

**GUI:**
- `eframe 0.31` — cross-platform native GUI framework (egui backend)
- `egui 0.31` — immediate-mode GUI library
- Renderer: `wgpu` (GPU-accelerated, with GL fallback)

**CLI Parsing:**
- `clap 4` with `derive` feature — `sentrux-bin/src/main.rs`

**Code Analysis:**
- `tree-sitter 0.25` — syntax tree parsing for multi-language support
- `tokei 12` — line/code/comment counting
- `oxc_resolver 11` — JavaScript/TypeScript import resolution

## Key Dependencies

**Critical:**
- `eframe 0.31` — GUI shell and wgpu renderer
- `egui 0.31` — all UI rendering
- `tree-sitter 0.25` — syntax parsing, loaded dynamically per language via plugins
- `git2 0.20` — local git repository status and history (libgit2 bindings)
- `rayon 1` — parallel scanning and analysis across files
- `crossbeam-channel 0.5` — messaging between scan threads and GUI
- `notify 7` + `notify-debouncer-mini 0.5` — filesystem watching for live reload
- `oxc_resolver 11` — JS/TS import path resolution
- `libloading 0.8` — dynamic loading of `.so`/`.dylib` language plugin grammars

**Infrastructure:**
- `dashmap 6` — concurrent hash maps for git status cache and scan data
- `serde 1` + `serde_json 1` — JSON serialization for MCP protocol and baseline files
- `toml 0.8` — parsing `.sentrux/rules.toml` and plugin manifests
- `ignore 0.4` — respects `.gitignore` during directory walking
- `dirs 6` — cross-platform home directory resolution (`~/.sentrux/`)
- `thiserror 2` — error type definitions
- `regex 1` — pattern matching in import resolution
- `rfd 0.15` — native file dialogs (GTK3 feature, Linux)
- `streaming-iterator 0.1.9` — tree-sitter query result iteration

## Features

**Cargo features used:**
- `sentrux-core`: `pro` feature — activated by optional `sentrux-pro` crate (private repository)
- `sentrux-bin`: `pro = ["sentrux-core/pro"]` — propagates pro feature through

Free tier always builds without `--features pro`. Pro tier compiled by a separate private crate.

## Configuration

**Runtime environment variables:**
- `SENTRUX_NO_UPDATE_CHECK=1` — disables daily version ping to `api.sentrux.dev`
- `WGPU_BACKEND=vulkan|gl|metal` — override GPU backend selection

**Project configuration:**
- `.sentrux/rules.toml` — architectural rules per project (constraints, layers, boundaries)
- `.sentrux/baseline.json` — saved metric baseline for `sentrux gate`
- `~/.sentrux/plugins/` — installed language plugins directory
- `~/.sentrux/last_update_check` — timestamp cache for daily update ping

**Build:**
- `Cargo.toml` workspace manifest at root
- Release profile: `opt-level = 3`, `lto = "thin"`
- No build scripts or codegen detected

## Platform Requirements

**Development:**
- Rust 1.94+ (stable)
- `cargo build --release`
- GTK3 headers on Linux (for `rfd` native dialogs)
- GPU driver with Vulkan, Metal, or OpenGL support for wgpu rendering

**Production targets:**
- macOS ARM64 (`darwin-arm64`) — primary supported target
- Linux x86_64 (`linux-x86_64`)
- macOS Intel not yet available (per `install.sh`)
- Distributed as a single static binary via GitHub Releases
- Homebrew install also supported (`brew upgrade sentrux`)

---

*Stack analysis: 2026-03-14*
