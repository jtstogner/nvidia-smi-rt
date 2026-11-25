use std::time::Duration;

use anyhow::Result;
use clap::Parser;

use nvidia_smi_rt::gui::{run_gui, GuiLaunchOptions};

/// Native Desktop GUI for Real-Time NVIDIA GPU Telemetry
#[derive(Parser, Debug)]
#[command(
    name = "nvidia-smi-rt-gui",
    author,
    version,
    about = "Real-time NVIDIA GPU telemetry desktop dashboard and process manager",
    long_about = None
)]
struct GuiCli {
    /// Telemetry polling interval in milliseconds (minimum 50ms)
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

    /// Initial window width in logical pixels
    #[arg(long, default_value_t = 1320.0)]
    width: f32,

    /// Initial window height in logical pixels
    #[arg(long, default_value_t = 860.0)]
    height: f32,
}

fn main() -> Result<()> {
    let cli = GuiCli::parse();
    let monitor_all = cli.all || cli.gpu.eq_ignore_ascii_case("all");
    let initial_gpu = if monitor_all {
        0
    } else {
        cli.gpu.parse::<u32>().unwrap_or_else(|_| {
            eprintln!("Invalid GPU index '{}', defaulting to GPU 0", cli.gpu);
            0
        })
    };

    run_gui(GuiLaunchOptions {
        interval: Duration::from_millis(cli.interval.max(50)),
        initial_gpu,
        monitor_all,
        force_cli: cli.no_nvml,
        width: cli.width.max(960.0),
        height: cli.height.max(620.0),
    })
}
