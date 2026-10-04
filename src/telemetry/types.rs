use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessType {
    Compute,
    Graphics,
    ComputeAndGraphics,
}

impl ProcessType {
    pub fn label(&self) -> &'static str {
        match self {
            ProcessType::Compute => "Compute",
            ProcessType::Graphics => "Graphics",
            ProcessType::ComputeAndGraphics => "C+G",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuProcess {
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
    pub proc_type: ProcessType,
    pub used_memory_bytes: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GpuSnapshot {
    pub gpu_index: u32,
    pub name: String,
    pub uuid: String,
    pub driver_version: String,
    pub cuda_version: String,
    pub pci_bus_id: String,
    pub pci_link_gen: Option<u32>,
    pub pci_link_max_gen: Option<u32>,
    pub pci_link_width: Option<u32>,
    pub pci_tx_kbytes_per_sec: Option<u64>,
    pub pci_rx_kbytes_per_sec: Option<u64>,
    pub util_gpu: f64,
    pub util_mem: f64,
    pub util_encoder: Option<f64>,
    pub util_decoder: Option<f64>,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub temp_c: u32,
    pub fan_speed_pct: Option<u32>,
    pub power_watts: f64,
    pub power_limit_watts: f64,
    pub clock_graphics_mhz: Option<u32>,
    pub clock_graphics_max_mhz: Option<u32>,
    pub clock_memory_mhz: Option<u32>,
    pub clock_memory_max_mhz: Option<u32>,
    pub persistence_mode: bool,
    pub perf_state: String,
    pub processes: Vec<GpuProcess>,
    pub timestamp: Instant,
}

impl GpuSnapshot {
    pub fn mem_used_percent(&self) -> f64 {
        if self.mem_total_bytes > 0 {
            (self.mem_used_bytes as f64 / self.mem_total_bytes as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn power_percent(&self) -> f64 {
        if self.power_limit_watts > 0.0 {
            (self.power_watts / self.power_limit_watts) * 100.0
        } else {
            0.0
        }
    }
}

/// Circular buffer storing time series telemetry points for charts.
#[derive(Debug, Clone)]
pub struct MetricSeries {
    pub points: VecDeque<(f64, f64)>, // (x_index, y_val)
    pub capacity: usize,
    pub next_x: f64,
}

impl MetricSeries {
    pub fn new(capacity: usize) -> Self {
        Self {
            points: VecDeque::with_capacity(capacity),
            capacity,
            next_x: 0.0,
        }
    }

    pub fn push(&mut self, val: f64) {
        if self.points.len() >= self.capacity {
            self.points.pop_front();
        }
        self.points.push_back((self.next_x, val));
        self.next_x += 1.0;
    }

    pub fn stats(&self) -> (f64, f64, f64, f64) {
        if self.points.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut sum = 0.0;
        let mut cur = 0.0;
        for &(_, y) in &self.points {
            if y < min {
                min = y;
            }
            if y > max {
                max = y;
            }
            sum += y;
            cur = y;
        }
        let avg = sum / self.points.len() as f64;
        (min, max, avg, cur)
    }

    /// Normalized (0..100) u64 array for sparklines
    pub fn to_sparkline_data(&self, max_scale: f64) -> Vec<u64> {
        self.points
            .iter()
            .map(|&(_, y)| {
                if max_scale <= 0.0 {
                    0
                } else {
                    let clamped = y.clamp(0.0, max_scale);
                    ((clamped / max_scale) * 100.0) as u64
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GpuHistory {
    pub capacity: usize,
    pub gpu_util: MetricSeries,
    pub mem_util: MetricSeries,
    pub mem_used_gib: MetricSeries,
    pub power_watts: MetricSeries,
    pub temp_c: MetricSeries,
    pub clock_graphics: MetricSeries,
}

impl GpuHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            gpu_util: MetricSeries::new(capacity),
            mem_util: MetricSeries::new(capacity),
            mem_used_gib: MetricSeries::new(capacity),
            power_watts: MetricSeries::new(capacity),
            temp_c: MetricSeries::new(capacity),
            clock_graphics: MetricSeries::new(capacity),
        }
    }

    pub fn record(&mut self, snap: &GpuSnapshot) {
        self.gpu_util.push(snap.util_gpu);
        self.mem_util.push(snap.mem_used_percent());
        self.mem_used_gib.push(snap.mem_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
        self.power_watts.push(snap.power_watts);
        self.temp_c.push(snap.temp_c as f64);
        if let Some(clk) = snap.clock_graphics_mhz {
            self.clock_graphics.push(clk as f64);
        }
    }
}
