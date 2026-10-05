mod app;
mod telemetry;
mod ui;
mod utils;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, IsTerminal};
use std::panic;
use std::time::{Duration, Instant};

use app::App;
use telemetry::{GpuSnapshot, TelemetryEngine};
use utils::{format_bytes, format_mhz, format_temp, format_watts};

/// High-Performance Real-Time NVIDIA GPU Telemetry Terminal Dashboard
#[derive(Parser, Debug)]
#[command(name = "nvidia-smi-rt", author, version, about = "Real-time NVIDIA GPU monitor with live Braille charts and process manager", long_about = None)]
struct Cli {
    /// Telemetry polling interval in milliseconds
    #[arg(short, long, default_value_t = 500)]
    interval: u64,

    /// Initial GPU index to monitor (e.g. 0, 1) or 'all' to monitor all GPUs
    #[arg(short, long, default_value = "0")]
    gpu: String,

    /// Monitor all GPUs simultaneously (alias: -g all)
    #[arg(short, long)]
    all: bool,

    /// Force using `nvidia-smi` CLI output instead of direct NVML C bindings
    #[arg(long)]
    no_nvml: bool,

    /// Print a single formatted telemetry snapshot and exit
    #[arg(short, long)]
    snapshot: bool,
}

fn print_snapshot(snap: &GpuSnapshot, engine: &TelemetryEngine) {
    println!("⚡ NVIDIA-SMI-RT Telemetry Snapshot ⚡");
    println!("GPU {}: {} | UUID: {}", snap.gpu_index, snap.name, snap.uuid);
    println!("Driver: {} | CUDA: {} | Backend: {}", snap.driver_version, snap.cuda_version, engine.backend_name());
    let pci_gen = snap.pci_link_gen.map(|g| format!("Gen{}", g)).unwrap_or_default();
    let pci_w = snap.pci_link_width.map(|w| format!("x{}", w)).unwrap_or_default();
    println!("PCIe: {} {} | Bus ID: {} | Perf State: {}", pci_gen, pci_w, snap.pci_bus_id, snap.perf_state);
    println!("Compute Util: {:.1}% | VRAM: {} / {} ({:.1}%)", snap.util_gpu, format_bytes(snap.mem_used_bytes), format_bytes(snap.mem_total_bytes), snap.mem_used_percent());
    println!("Power Draw:   {} / {} ({:.1}%)", format_watts(snap.power_watts), format_watts(snap.power_limit_watts), snap.power_percent());
    println!("Thermals:     {} | Fan: {}", format_temp(snap.temp_c), snap.fan_speed_pct.map(|f| format!("{}%", f)).unwrap_or_else(|| "N/A".to_string()));
    let gfx_clk = snap.clock_graphics_mhz.map(format_mhz).unwrap_or_else(|| "N/A".to_string());
    let mem_clk = snap.clock_memory_mhz.map(format_mhz).unwrap_or_else(|| "N/A".to_string());
    println!("Clocks:       Graphics: {} | Memory: {}", gfx_clk, mem_clk);
    println!("\nActive Processes ({}):", snap.processes.len());
    println!("  {:<8} {:<10} {:<14} {:<8} {}", "PID", "TYPE", "VRAM", "%", "NAME");
    println!("  {}", "-".repeat(60));
    for p in &snap.processes {
        let pct = (p.used_memory_bytes as f64 / snap.mem_total_bytes as f64) * 100.0;
        println!("  {:<8} {:<10} {:<14} {:<7.1}% {}", p.pid, p.proc_type.label(), format_bytes(p.used_memory_bytes), pct, p.name);
    }
}

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let engine = TelemetryEngine::init(cli.no_nvml)?;

    let is_all = cli.all || cli.gpu.eq_ignore_ascii_case("all");
    let initial_gpu = if is_all {
        0
    } else {
        cli.gpu.parse::<u32>().unwrap_or_else(|_| {
            eprintln!("Invalid GPU index '{}', defaulting to GPU 0", cli.gpu);
            0
        })
    };

    if cli.snapshot || !stdout().is_terminal() {
        if is_all {
            let count = engine.device_count().max(1);
            for i in 0..count {
                match engine.get_snapshot(i) {
                    Ok(snap) => {
                        print_snapshot(&snap, &engine);
                        if i + 1 < count {
                            println!("\n{}\n", "━".repeat(60));
                        }
                    }
                    Err(e) => eprintln!("Error querying GPU {}: {}", i, e),
                }
            }
        } else {
            let snap = engine.get_snapshot(initial_gpu)?;
            print_snapshot(&snap, &engine);
        }
        return Ok(());
    }

    setup_panic_hook();
    let interval = Duration::from_millis(cli.interval.max(50));

    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(engine, initial_gpu, interval, is_all);

    let tick_rate = Duration::from_millis(30); // 33 FPS UI loop
    let mut last_tick = Instant::now();

    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Ignore key release events on Windows/Crossterm
                if key.kind == crossterm::event::KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        // Periodic telemetry sample
        if !app.paused && app.last_update.elapsed() >= app.interval {
            app.refresh_telemetry();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
