use anyhow::{Context, Result};
use nvml_wrapper::enum_wrappers::device::{Clock, PcieUtilCounter, TemperatureSensor};
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::Nvml;
use std::collections::HashMap;
use std::time::Instant;

use super::types::{GpuProcess, GpuSnapshot, ProcessType};
use crate::utils::get_process_info_from_proc;

pub struct NvmlBackend {
    nvml: Nvml,
}

impl NvmlBackend {
    pub fn new() -> Result<Self> {
        let nvml = Nvml::init().context("Failed to initialize NVML library")?;
        Ok(Self { nvml })
    }

    pub fn device_count(&self) -> u32 {
        self.nvml.device_count().unwrap_or(0)
    }

    pub fn get_snapshot(&self, gpu_index: u32) -> Result<GpuSnapshot> {
        let device = self
            .nvml
            .device_by_index(gpu_index)
            .with_context(|| format!("Failed to get device by index {}", gpu_index))?;

        let driver_version = self
            .nvml
            .sys_driver_version()
            .unwrap_or_else(|_| "Unknown".to_string());

        let cuda_version = self
            .nvml
            .sys_cuda_driver_version()
            .map(|v| format!("{}.{}", v / 1000, (v % 1000) / 10))
            .unwrap_or_else(|_| "Unknown".to_string());

        let name = device.name().unwrap_or_else(|_| format!("GPU {}", gpu_index));
        let uuid = device.uuid().unwrap_or_default();

        let pci_info = device.pci_info().ok();
        let pci_bus_id = pci_info
            .as_ref()
            .map(|p| p.bus_id.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let pci_link_gen = device.current_pcie_link_gen().ok();
        let pci_link_max_gen = device.max_pcie_link_gen().ok();
        let pci_link_width = device.current_pcie_link_width().ok();

        // PCIe throughput in KB/s
        let pci_tx_kbytes_per_sec = device
            .pcie_throughput(PcieUtilCounter::Send)
            .ok()
            .map(|v| v as u64);
        let pci_rx_kbytes_per_sec = device
            .pcie_throughput(PcieUtilCounter::Receive)
            .ok()
            .map(|v| v as u64);

        let util = device.utilization_rates().ok();
        let util_gpu = util.as_ref().map(|u| u.gpu as f64).unwrap_or(0.0);
        let util_mem = util.as_ref().map(|u| u.memory as f64).unwrap_or(0.0);

        let util_encoder = device
            .encoder_utilization()
            .ok()
            .map(|u| u.utilization as f64);
        let util_decoder = device
            .decoder_utilization()
            .ok()
            .map(|u| u.utilization as f64);

        let mem = device.memory_info().ok();
        let mem_used_bytes = mem.as_ref().map(|m| m.used).unwrap_or(0);
        let mem_total_bytes = mem.as_ref().map(|m| m.total).unwrap_or(0);

        let temp_c = device
            .temperature(TemperatureSensor::Gpu)
            .unwrap_or(0);

        let fan_speed_pct = device.fan_speed(0).ok();

        let power_watts = device
            .power_usage()
            .map(|mw| mw as f64 / 1000.0)
            .unwrap_or(0.0);

        let power_limit_watts = device
            .enforced_power_limit()
            .or_else(|_| device.power_management_limit())
            .map(|mw| mw as f64 / 1000.0)
            .unwrap_or(0.0);

        let clock_graphics_mhz = device.clock_info(Clock::Graphics).ok();
        let clock_graphics_max_mhz = device.max_clock_info(Clock::Graphics).ok();
        let clock_memory_mhz = device.clock_info(Clock::Memory).ok();
        let clock_memory_max_mhz = device.max_clock_info(Clock::Memory).ok();

        let persistence_mode = device.is_in_persistent_mode().unwrap_or(false);
        let perf_state = device
            .performance_state()
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|_| "P0".to_string());

        // Process discovery
        let mut proc_map: HashMap<u32, (ProcessType, u64)> = HashMap::new();

        if let Ok(compute_procs) = device.running_compute_processes() {
            for cp in compute_procs {
                let bytes = match cp.used_gpu_memory {
                    UsedGpuMemory::Used(b) => b,
                    UsedGpuMemory::Unavailable => 0,
                };
                proc_map.insert(cp.pid, (ProcessType::Compute, bytes));
            }
        }

        if let Ok(graphics_procs) = device.running_graphics_processes() {
            for gp in graphics_procs {
                let bytes = match gp.used_gpu_memory {
                    UsedGpuMemory::Used(b) => b,
                    UsedGpuMemory::Unavailable => 0,
                };
                proc_map
                    .entry(gp.pid)
                    .and_modify(|(t, existing_b)| {
                        *t = ProcessType::ComputeAndGraphics;
                        if bytes > *existing_b {
                            *existing_b = bytes;
                        }
                    })
                    .or_insert((ProcessType::Graphics, bytes));
            }
        }

        let mut processes = Vec::new();
        for (pid, (proc_type, used_memory_bytes)) in proc_map {
            let (name, cmdline) = get_process_info_from_proc(pid);
            processes.push(GpuProcess {
                pid,
                name,
                cmdline,
                proc_type,
                used_memory_bytes,
            });
        }

        // Sort descending by memory usage
        processes.sort_by(|a, b| b.used_memory_bytes.cmp(&a.used_memory_bytes));

        Ok(GpuSnapshot {
            gpu_index,
            name,
            uuid,
            driver_version,
            cuda_version,
            pci_bus_id,
            pci_link_gen,
            pci_link_max_gen,
            pci_link_width,
            pci_tx_kbytes_per_sec,
            pci_rx_kbytes_per_sec,
            util_gpu,
            util_mem,
            util_encoder,
            util_decoder,
            mem_used_bytes,
            mem_total_bytes,
            temp_c,
            fan_speed_pct,
            power_watts,
            power_limit_watts,
            clock_graphics_mhz,
            clock_graphics_max_mhz,
            clock_memory_mhz,
            clock_memory_max_mhz,
            persistence_mode,
            perf_state,
            processes,
            timestamp: Instant::now(),
        })
    }
}
