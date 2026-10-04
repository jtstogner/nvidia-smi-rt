use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

use super::types::{GpuProcess, GpuSnapshot, ProcessType};
use crate::utils::get_process_info_from_proc;

pub struct CliBackend;

impl CliBackend {
    pub fn new() -> Result<Self> {
        let output = Command::new("nvidia-smi")
            .arg("-L")
            .output()
            .context("nvidia-smi command not found in PATH")?;

        if !output.status.success() {
            bail!("nvidia-smi -L returned error status");
        }
        Ok(Self)
    }

    pub fn device_count(&self) -> u32 {
        let output = Command::new("nvidia-smi").arg("-L").output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines().filter(|l| l.starts_with("GPU ")).count() as u32
        } else {
            0
        }
    }

    pub fn get_snapshot(&self, gpu_index: u32) -> Result<GpuSnapshot> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=index,name,uuid,driver_version,utilization.gpu,utilization.memory,memory.used,memory.total,temperature.gpu,fan.speed,power.draw,power.limit,clocks.current.graphics,clocks.max.graphics,pci.bus_id",
                "--format=csv,noheader,nounits",
                "-i",
                &gpu_index.to_string(),
            ])
            .output()
            .context("Failed to execute nvidia-smi query")?;

        if !output.status.success() {
            bail!("nvidia-smi returned an error");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next().context("Empty output from nvidia-smi")?;
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 15 {
            bail!("Incomplete CSV fields from nvidia-smi: got {}", parts.len());
        }

        let name = parts[1].to_string();
        let uuid = parts[2].to_string();
        let driver_version = parts[3].to_string();
        let util_gpu = parts[4].parse::<f64>().unwrap_or(0.0);
        let util_mem = parts[5].parse::<f64>().unwrap_or(0.0);
        let mem_used_bytes = parts[6].parse::<u64>().unwrap_or(0) * 1024 * 1024;
        let mem_total_bytes = parts[7].parse::<u64>().unwrap_or(0) * 1024 * 1024;
        let temp_c = parts[8].parse::<u32>().unwrap_or(0);
        let fan_speed_pct = parts[9].parse::<u32>().ok();
        let power_watts = parts[10].parse::<f64>().unwrap_or(0.0);
        let power_limit_watts = parts[11].parse::<f64>().unwrap_or(0.0);
        let clock_graphics_mhz = parts[12].parse::<u32>().ok();
        let clock_graphics_max_mhz = parts[13].parse::<u32>().ok();
        let pci_bus_id = parts[14].to_string();

        // Query processes
        let mut proc_map: HashMap<u32, (ProcessType, u64)> = HashMap::new();

        if let Ok(c_out) = Command::new("nvidia-smi")
            .args([
                "--query-compute-apps=pid,process_name,used_memory",
                "--format=csv,noheader,nounits",
                "-i",
                &gpu_index.to_string(),
            ])
            .output()
        {
            let text = String::from_utf8_lossy(&c_out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    if let Ok(pid) = parts[0].parse::<u32>() {
                        let used_bytes = parts[2].parse::<u64>().unwrap_or(0) * 1024 * 1024;
                        proc_map.insert(pid, (ProcessType::Compute, used_bytes));
                    }
                }
            }
        }

        if let Ok(g_out) = Command::new("nvidia-smi")
            .args([
                "--query-graphics-apps=pid,process_name,used_memory",
                "--format=csv,noheader,nounits",
                "-i",
                &gpu_index.to_string(),
            ])
            .output()
        {
            let text = String::from_utf8_lossy(&g_out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    if let Ok(pid) = parts[0].parse::<u32>() {
                        let used_bytes = parts[2].parse::<u64>().unwrap_or(0) * 1024 * 1024;
                        proc_map
                            .entry(pid)
                            .and_modify(|(t, existing)| {
                                *t = ProcessType::ComputeAndGraphics;
                                if used_bytes > *existing {
                                    *existing = used_bytes;
                                }
                            })
                            .or_insert((ProcessType::Graphics, used_bytes));
                    }
                }
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
        processes.sort_by(|a, b| b.used_memory_bytes.cmp(&a.used_memory_bytes));

        Ok(GpuSnapshot {
            gpu_index,
            name,
            uuid,
            driver_version,
            cuda_version: "N/A".to_string(),
            pci_bus_id,
            pci_link_gen: None,
            pci_link_max_gen: None,
            pci_link_width: None,
            pci_tx_kbytes_per_sec: None,
            pci_rx_kbytes_per_sec: None,
            util_gpu,
            util_mem,
            util_encoder: None,
            util_decoder: None,
            mem_used_bytes,
            mem_total_bytes,
            temp_c,
            fan_speed_pct,
            power_watts,
            power_limit_watts,
            clock_graphics_mhz,
            clock_graphics_max_mhz,
            clock_memory_mhz: None,
            clock_memory_max_mhz: None,
            persistence_mode: false,
            perf_state: "N/A".to_string(),
            processes,
            timestamp: Instant::now(),
        })
    }
}
