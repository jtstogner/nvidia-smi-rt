pub mod cli_backend;
pub mod nvml_backend;
pub mod types;

use anyhow::Result;
use cli_backend::CliBackend;
use nvml_backend::NvmlBackend;
pub use types::*;

pub enum TelemetryEngine {
    Nvml(NvmlBackend),
    Cli(CliBackend),
}

impl TelemetryEngine {
    pub fn init(force_cli: bool) -> Result<Self> {
        if !force_cli {
            match NvmlBackend::new() {
                Ok(backend) => return Ok(TelemetryEngine::Nvml(backend)),
                Err(e) => {
                    eprintln!("Warning: NVML initialization failed ({}). Falling back to nvidia-smi CLI...", e);
                }
            }
        }
        let cli = CliBackend::new()?;
        Ok(TelemetryEngine::Cli(cli))
    }

    pub fn backend_name(&self) -> &'static str {
        match self {
            TelemetryEngine::Nvml(_) => "NVML (Native C Library)",
            TelemetryEngine::Cli(_) => "nvidia-smi CLI (Fallback)",
        }
    }

    pub fn device_count(&self) -> u32 {
        match self {
            TelemetryEngine::Nvml(b) => b.device_count(),
            TelemetryEngine::Cli(b) => b.device_count(),
        }
    }

    pub fn get_snapshot(&self, gpu_index: u32) -> Result<GpuSnapshot> {
        match self {
            TelemetryEngine::Nvml(b) => b.get_snapshot(gpu_index),
            TelemetryEngine::Cli(b) => b.get_snapshot(gpu_index),
        }
    }
}
