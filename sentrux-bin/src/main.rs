//! Sentrux binary — GUI and CLI entry points.
//!
//! All logic lives in `sentrux-core`. This crate is just the entry point
//! that wires together the three modes:
//! - GUI mode (default): interactive treemap/blueprint visualizer
//! - Check mode (`sentrux check [path]`): delegates to `pmat quality-gate`
//! - Gate mode (`sentrux gate [path]`): delegates to `pmat tdg --min-grade`

use clap::{Parser, Subcommand};
use sentrux_core::app;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

fn version_string() -> &'static str {
    use std::sync::OnceLock;
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        let edition = if sentrux_core::license::current_tier() >= sentrux_core::license::Tier::Pro { "Pro" } else { "Free" };
        format!("{} ({})", env!("CARGO_PKG_VERSION"), edition)
    })
}

#[derive(Parser)]
#[command(
    name = "sentrux",
    about = "Live codebase visualization and structural quality gate",
    version = version_string(),
    arg_required_else_help = false,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Directory to open in the GUI
    #[arg(global = false)]
    path: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Run PMAT quality gate on the project (delegates to `pmat quality-gate`)
    Check {
        /// Directory to check
        #[arg(default_value = ".")]
        path: String,
    },

    /// Run PMAT TDG grade gate on the project (delegates to `pmat tdg --min-grade`)
    Gate {
        /// Directory to gate
        #[arg(default_value = ".")]
        path: String,
    },

    /// Open the GUI with a pre-loaded directory
    Scan {
        /// Directory to visualize
        path: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    // Non-blocking update check (once per day, background thread)
    app::update_check::check_for_updates_async(env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Check { path }) => {
            std::process::exit(run_check(&path));
        }
        Some(Command::Gate { path }) => {
            std::process::exit(run_gate(&path));
        }
        Some(Command::Scan { path }) => {
            run_gui(path)
        }
        None => {
            run_gui(cli.path)
        }
    }
}

// ---------------------------------------------------------------------------
// Check — delegates to pmat quality-gate
// ---------------------------------------------------------------------------

/// Run PMAT quality gate from CLI. Returns exit code.
fn run_check(path: &str) -> i32 {
    eprintln!("[check] Running PMAT quality gate on {path}");

    let status = std::process::Command::new("pmat")
        .args(["quality-gate", "--format", "json", "--fail-on-violation", "--path", path])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    match status {
        Ok(s) if s.success() => {
            eprintln!("[check] Quality gate passed");
            0
        }
        Ok(s) => {
            eprintln!("[check] Quality gate failed (exit {})", s.code().unwrap_or(-1));
            1
        }
        Err(e) => {
            eprintln!("[check] Failed to run pmat: {e}");
            eprintln!("[check] Install PMAT: cargo install pmat");
            1
        }
    }
}

// ---------------------------------------------------------------------------
// Gate — delegates to pmat tdg --min-grade
// ---------------------------------------------------------------------------

/// Run PMAT TDG grade gate from CLI. Returns exit code.
fn run_gate(path: &str) -> i32 {
    eprintln!("[gate] Running PMAT TDG grade gate on {path}");

    // Default minimum grade: C (configurable in future)
    let status = std::process::Command::new("pmat")
        .args(["tdg", "--min-grade", "C", "--format", "json", "--path", path])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    match status {
        Ok(s) if s.success() => {
            eprintln!("[gate] Grade gate passed");
            0
        }
        Ok(s) => {
            eprintln!("[gate] Grade gate failed (exit {})", s.code().unwrap_or(-1));
            1
        }
        Err(e) => {
            eprintln!("[gate] Failed to run pmat: {e}");
            eprintln!("[gate] Install PMAT: cargo install pmat");
            1
        }
    }
}

// ---------------------------------------------------------------------------
// GUI
// ---------------------------------------------------------------------------

/// Probe which wgpu backends have usable GPU adapters on this system.
/// Returns only backends that actually have hardware support, avoiding
/// blind attempts that panic on unsupported drivers.
fn probe_available_backends() -> Vec<eframe::wgpu::Backends> {
    let candidates = [
        ("Primary+GL", eframe::wgpu::Backends::PRIMARY | eframe::wgpu::Backends::GL),
        ("GL-only",    eframe::wgpu::Backends::GL),
        ("Primary",    eframe::wgpu::Backends::PRIMARY),
    ];

    let mut available = Vec::new();
    for (label, backends) in &candidates {
        let instance = eframe::wgpu::Instance::new(&eframe::wgpu::InstanceDescriptor {
            backends: *backends,
            ..Default::default()
        });
        let adapters: Vec<_> = instance.enumerate_adapters(eframe::wgpu::Backends::all());
        if !adapters.is_empty() {
            eprintln!("[gpu] probe {label}: {} adapter(s) found", adapters.len());
            available.push(*backends);
        } else {
            eprintln!("[gpu] probe {label}: no adapters");
        }
    }
    available
}

fn run_gui(path: Option<String>) -> eframe::Result<()> {
    let initial_path = path
        .map(|p| {
            std::path::Path::new(&p)
                .canonicalize()
                .map(|c| c.to_string_lossy().to_string())
                .unwrap_or(p)
        })
        .filter(|p| std::path::Path::new(p).is_dir());

    // Determine backends: respect user override, otherwise probe hardware.
    let env_backends = eframe::wgpu::Backends::from_env();
    let backend_attempts: Vec<eframe::wgpu::Backends> = if let Some(user_choice) = env_backends {
        // User explicitly chose via WGPU_BACKEND — respect it, no fallback
        vec![user_choice]
    } else {
        let probed = probe_available_backends();
        if probed.is_empty() {
            eprintln!("[gpu] no GPU adapters found on this system");
            eprintln!("[gpu] hint: try setting WGPU_BACKEND=vulkan or WGPU_BACKEND=gl");
            std::process::exit(1);
        }
        probed
    };

    let title = if sentrux_core::license::current_tier() >= sentrux_core::license::Tier::Pro { "Sentrux Pro" } else { "sentrux" };

    for (i, backends) in backend_attempts.iter().enumerate() {
        eprintln!("[gpu] attempt {}/{}: backends {:?}", i + 1, backend_attempts.len(), backends);

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1600.0, 1000.0])
                .with_maximized(true)
                .with_title(title),
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(eframe::egui_wgpu::WgpuSetupCreateNew {
                    instance_descriptor: eframe::wgpu::InstanceDescriptor {
                        backends: *backends,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let path_clone = initial_path.clone();
        // catch_unwind as safety net: wgpu can panic on surface creation
        // even when adapter enumeration succeeded (driver bugs, missing DRI3)
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            eframe::run_native(
                "Sentrux",
                options,
                Box::new(move |cc| Ok(Box::new(app::SentruxApp::new(cc, path_clone)))),
            )
        }));

        match result {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(e)) => {
                eprintln!("[gpu] backend {:?} failed: {e}", backends);
            }
            Err(_panic) => {
                eprintln!("[gpu] backend {:?} panicked (driver issue)", backends);
            }
        }

        if i + 1 == backend_attempts.len() {
            eprintln!("[gpu] all backends exhausted");
            eprintln!("[gpu] hint: try setting WGPU_BACKEND=vulkan or WGPU_BACKEND=gl");
            std::process::exit(1);
        }
    }
    Ok(())
}
