use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::telemetry::{GpuHistory, GpuProcess, GpuSnapshot};
use crate::utils::terminate_process;

use super::poller::{PollerCommand, TelemetryPoller};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiTab {
    AllGpus,
    Overview,
    Charts,
    Processes,
}

impl GuiTab {
    pub fn all() -> &'static [GuiTab] {
        &[
            GuiTab::AllGpus,
            GuiTab::Overview,
            GuiTab::Charts,
            GuiTab::Processes,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            GuiTab::AllGpus => "[0] All GPUs",
            GuiTab::Overview => "[1] Overview",
            GuiTab::Charts => "[2] Detailed Charts",
            GuiTab::Processes => "[3] Process Manager",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            GuiTab::AllGpus => GuiTab::Overview,
            GuiTab::Overview => GuiTab::Charts,
            GuiTab::Charts => GuiTab::Processes,
            GuiTab::Processes => GuiTab::AllGpus,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            GuiTab::AllGpus => GuiTab::Processes,
            GuiTab::Overview => GuiTab::AllGpus,
            GuiTab::Charts => GuiTab::Overview,
            GuiTab::Processes => GuiTab::Charts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiProcessSort {
    MemoryDesc,
    MemoryAsc,
    Pid,
    Name,
}

impl GuiProcessSort {
    pub fn all() -> &'static [GuiProcessSort] {
        &[
            GuiProcessSort::MemoryDesc,
            GuiProcessSort::MemoryAsc,
            GuiProcessSort::Pid,
            GuiProcessSort::Name,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            GuiProcessSort::MemoryDesc => "VRAM (High to Low)",
            GuiProcessSort::MemoryAsc => "VRAM (Low to High)",
            GuiProcessSort::Pid => "PID",
            GuiProcessSort::Name => "Name",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            GuiProcessSort::MemoryDesc => GuiProcessSort::MemoryAsc,
            GuiProcessSort::MemoryAsc => GuiProcessSort::Pid,
            GuiProcessSort::Pid => GuiProcessSort::Name,
            GuiProcessSort::Name => GuiProcessSort::MemoryDesc,
        }
    }
}

pub struct FleetSummary {
    pub device_count: u32,
    pub sampled_count: u32,
    pub avg_util: f64,
    pub total_vram_used: u64,
    pub total_vram_cap: u64,
    pub total_power_watts: f64,
    pub total_power_limit: f64,
    pub max_temp: u32,
    pub total_procs: usize,
}

pub struct GuiApp {
    pub poller: TelemetryPoller,
    pub device_count: u32,
    pub selected_gpu: u32,
    pub monitor_all: bool,
    pub current_snapshot: Option<GpuSnapshot>,
    pub all_snapshots: HashMap<u32, GpuSnapshot>,
    pub histories: HashMap<u32, GpuHistory>,
    pub active_tab: GuiTab,
    pub interval: Duration,
    pub paused: bool,
    pub sample_count: u64,
    pub process_sort: GuiProcessSort,
    pub process_filter: String,
    pub selected_pid: Option<u32>,
    pub show_help_modal: bool,
    pub kill_confirm_proc: Option<GpuProcess>,
    pub charts_show_clock_mode: bool,
    pub status_message: Option<(String, Instant)>,
}

impl GuiApp {
    pub fn new(
        poller: TelemetryPoller,
        initial_gpu: u32,
        interval: Duration,
        monitor_all: bool,
    ) -> Self {
        let device_count = poller.device_count.max(1);
        let selected_gpu = initial_gpu.min(device_count.saturating_sub(1));

        let mut histories = HashMap::new();
        for i in 0..device_count {
            histories.insert(i, GpuHistory::new(300));
        }

        let active_tab = if monitor_all {
            GuiTab::AllGpus
        } else {
            GuiTab::Overview
        };

        let mut app = Self {
            poller,
            device_count,
            selected_gpu,
            monitor_all,
            current_snapshot: None,
            all_snapshots: HashMap::new(),
            histories,
            active_tab,
            interval: interval.max(Duration::from_millis(50)),
            paused: false,
            sample_count: 0,
            process_sort: GuiProcessSort::MemoryDesc,
            process_filter: String::new(),
            selected_pid: None,
            show_help_modal: false,
            kill_confirm_proc: None,
            charts_show_clock_mode: false,
            status_message: None,
        };

        app.drain_samples();
        app
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn drain_samples(&mut self) {
        while let Some(batch) = self.poller.try_recv() {
            let had_snapshots = !batch.snapshots.is_empty();
            for (gpu_idx, snap) in batch.snapshots {
                let history = self
                    .histories
                    .entry(gpu_idx)
                    .or_insert_with(|| GpuHistory::new(300));
                history.record(&snap);
                if gpu_idx == self.selected_gpu {
                    self.current_snapshot = Some(snap.clone());
                }
                self.all_snapshots.insert(gpu_idx, snap);
            }
            if had_snapshots {
                self.sample_count += 1;
            }
            if let Some(err) = batch.error {
                self.set_status(err);
            }
        }
    }

    fn poll_all_needed(&self) -> bool {
        self.monitor_all || self.active_tab == GuiTab::AllGpus
    }

    pub fn sync_poller_target(&self) {
        self.poller.send(PollerCommand::SetTarget {
            selected_gpu: self.selected_gpu,
            poll_all: self.poll_all_needed(),
        });
    }

    pub fn select_gpu(&mut self, idx: u32) {
        let clamped = idx.min(self.device_count.saturating_sub(1));
        if clamped != self.selected_gpu {
            self.selected_gpu = clamped;
            self.current_snapshot = self.all_snapshots.get(&clamped).cloned();
            self.selected_pid = None;
            self.sync_poller_target();
        }
    }

    pub fn set_tab(&mut self, tab: GuiTab) {
        if self.active_tab != tab {
            self.active_tab = tab;
            self.sync_poller_target();
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.paused = paused;
            self.poller.send(PollerCommand::SetPaused(paused));
            if paused {
                self.set_status("PAUSED - Telemetry polling frozen");
            } else {
                self.set_status("RESUMED - Telemetry polling active");
            }
        }
    }

    pub fn set_interval(&mut self, interval: Duration) {
        let clamped_ms = (interval.as_millis() as u64).clamp(50, 5000);
        let new_interval = Duration::from_millis(clamped_ms);
        if self.interval != new_interval {
            self.interval = new_interval;
            self.poller.send(PollerCommand::SetInterval(new_interval));
            self.set_status(format!("Interval set to {} ms", clamped_ms));
        }
    }

    pub fn force_refresh(&mut self) {
        self.poller.send(PollerCommand::ForceRefresh);
        self.set_status("Refreshing telemetry...");
    }

    pub fn terminate_selected(&mut self, pid: u32) {
        match terminate_process(pid) {
            Ok(()) => {
                self.set_status(format!("Sent SIGTERM to PID {}", pid));
                self.poller.send(PollerCommand::ForceRefresh);
            }
            Err(e) => {
                self.set_status(format!("Failed to terminate PID {}: {}", pid, e));
            }
        }
    }

    pub fn filtered_sorted_processes(&self) -> Vec<GpuProcess> {
        let Some(snap) = &self.current_snapshot else {
            return Vec::new();
        };
        filter_and_sort_processes(&snap.processes, &self.process_filter, self.process_sort)
    }

    pub fn fleet_summary(&self) -> FleetSummary {
        compute_fleet_summary(self.device_count, &self.all_snapshots)
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Modal shortcuts always take priority
        if self.show_help_modal {
            let close = ctx.input(|i| {
                i.key_pressed(egui::Key::Escape)
                    || i.key_pressed(egui::Key::Enter)
                    || i.key_pressed(egui::Key::F1)
            });
            if close {
                self.show_help_modal = false;
            }
            return;
        }

        if let Some(target) = self.kill_confirm_proc.clone() {
            let (confirm, cancel) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Y),
                    i.key_pressed(egui::Key::N) || i.key_pressed(egui::Key::Escape),
                )
            });
            if confirm {
                self.terminate_selected(target.pid);
                self.kill_confirm_proc = None;
            } else if cancel {
                self.kill_confirm_proc = None;
                self.set_status("Kill cancelled");
            }
            return;
        }

        if ctx.egui_wants_keyboard_input() {
            return;
        }

        enum ShortcutAction {
            None,
            Tab(GuiTab),
            NextTab,
            PrevTab,
            TogglePause,
            SpeedUp,
            SlowDown,
            PrevGpu,
            NextGpu,
            Refresh,
            CycleSort,
            OpenHelp,
        }

        let action = ctx.input(|i| {
            if i.key_pressed(egui::Key::F1) || i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::Questionmark) {
                ShortcutAction::OpenHelp
            } else if i.key_pressed(egui::Key::Space) {
                ShortcutAction::TogglePause
            } else if i.key_pressed(egui::Key::Tab) {
                if i.modifiers.shift {
                    ShortcutAction::PrevTab
                } else {
                    ShortcutAction::NextTab
                }
            } else if i.key_pressed(egui::Key::Num0) || i.key_pressed(egui::Key::A) {
                ShortcutAction::Tab(GuiTab::AllGpus)
            } else if i.key_pressed(egui::Key::Num1) {
                ShortcutAction::Tab(GuiTab::Overview)
            } else if i.key_pressed(egui::Key::Num2) {
                ShortcutAction::Tab(GuiTab::Charts)
            } else if i.key_pressed(egui::Key::Num3) {
                ShortcutAction::Tab(GuiTab::Processes)
            } else if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                ShortcutAction::SpeedUp
            } else if i.key_pressed(egui::Key::Minus) {
                ShortcutAction::SlowDown
            } else if i.key_pressed(egui::Key::ArrowLeft) {
                ShortcutAction::PrevGpu
            } else if i.key_pressed(egui::Key::ArrowRight) {
                ShortcutAction::NextGpu
            } else if i.key_pressed(egui::Key::R) {
                ShortcutAction::Refresh
            } else if i.key_pressed(egui::Key::S) {
                ShortcutAction::CycleSort
            } else {
                ShortcutAction::None
            }
        });

        match action {
            ShortcutAction::None => {}
            ShortcutAction::Tab(tab) => self.set_tab(tab),
            ShortcutAction::NextTab => self.set_tab(self.active_tab.next()),
            ShortcutAction::PrevTab => self.set_tab(self.active_tab.prev()),
            ShortcutAction::TogglePause => self.set_paused(!self.paused),
            ShortcutAction::SpeedUp => {
                let ms = self.interval.as_millis() as u64;
                self.set_interval(Duration::from_millis(ms.saturating_sub(100).max(100)));
            }
            ShortcutAction::SlowDown => {
                let ms = self.interval.as_millis() as u64;
                self.set_interval(Duration::from_millis((ms + 100).min(5000)));
            }
            ShortcutAction::PrevGpu => {
                if self.device_count > 1 {
                    let prev = (self.selected_gpu + self.device_count - 1) % self.device_count;
                    self.select_gpu(prev);
                }
            }
            ShortcutAction::NextGpu => {
                if self.device_count > 1 {
                    let next = (self.selected_gpu + 1) % self.device_count;
                    self.select_gpu(next);
                }
            }
            ShortcutAction::Refresh => self.force_refresh(),
            ShortcutAction::CycleSort => {
                if self.active_tab == GuiTab::Processes {
                    self.process_sort = self.process_sort.next();
                    self.set_status(format!("Sort by: {}", self.process_sort.label()));
                }
            }
            ShortcutAction::OpenHelp => {
                self.show_help_modal = true;
            }
        }
    }
}

pub fn filter_and_sort_processes(
    processes: &[GpuProcess],
    filter: &str,
    sort: GuiProcessSort,
) -> Vec<GpuProcess> {
    let query = filter.trim().to_lowercase();
    let mut procs: Vec<GpuProcess> = if query.is_empty() {
        processes.to_vec()
    } else {
        processes
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query)
                    || p.cmdline.to_lowercase().contains(&query)
                    || p.pid.to_string().contains(&query)
            })
            .cloned()
            .collect()
    };

    match sort {
        GuiProcessSort::MemoryDesc => {
            procs.sort_by(|a, b| b.used_memory_bytes.cmp(&a.used_memory_bytes))
        }
        GuiProcessSort::MemoryAsc => {
            procs.sort_by(|a, b| a.used_memory_bytes.cmp(&b.used_memory_bytes))
        }
        GuiProcessSort::Pid => procs.sort_by_key(|p| p.pid),
        GuiProcessSort::Name => {
            procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }
    }
    procs
}

pub fn compute_fleet_summary(
    device_count: u32,
    all_snapshots: &HashMap<u32, GpuSnapshot>,
) -> FleetSummary {
    let mut total_util = 0.0;
    let mut total_vram_used = 0u64;
    let mut total_vram_cap = 0u64;
    let mut total_power_watts = 0.0;
    let mut total_power_limit = 0.0;
    let mut max_temp = 0u32;
    let mut total_procs = 0usize;
    let mut sampled_count = 0u32;

    for i in 0..device_count {
        if let Some(snap) = all_snapshots.get(&i) {
            total_util += snap.util_gpu;
            total_vram_used += snap.mem_used_bytes;
            total_vram_cap += snap.mem_total_bytes;
            total_power_watts += snap.power_watts;
            total_power_limit += snap.power_limit_watts;
            if snap.temp_c > max_temp {
                max_temp = snap.temp_c;
            }
            total_procs += snap.processes.len();
            sampled_count += 1;
        }
    }

    let avg_util = if sampled_count > 0 {
        total_util / sampled_count as f64
    } else {
        0.0
    };

    FleetSummary {
        device_count,
        sampled_count,
        avg_util,
        total_vram_used,
        total_vram_cap,
        total_power_watts,
        total_power_limit,
        max_temp,
        total_procs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::ProcessType;

    fn sample_proc(pid: u32, name: &str, cmdline: &str, mem: u64) -> GpuProcess {
        GpuProcess {
            pid,
            name: name.to_string(),
            cmdline: cmdline.to_string(),
            proc_type: ProcessType::Compute,
            used_memory_bytes: mem,
        }
    }

    fn sample_snapshot(idx: u32, util: f64, mem_used: u64, mem_total: u64, power: f64, limit: f64, temp: u32, procs: Vec<GpuProcess>) -> GpuSnapshot {
        GpuSnapshot {
            gpu_index: idx,
            name: format!("GPU {}", idx),
            uuid: format!("UUID-{}", idx),
            driver_version: "560.0".to_string(),
            cuda_version: "12.6".to_string(),
            pci_bus_id: "0000:01:00.0".to_string(),
            pci_link_gen: Some(4),
            pci_link_max_gen: Some(4),
            pci_link_width: Some(16),
            pci_tx_kbytes_per_sec: None,
            pci_rx_kbytes_per_sec: None,
            util_gpu: util,
            util_mem: 10.0,
            util_encoder: None,
            util_decoder: None,
            mem_used_bytes: mem_used,
            mem_total_bytes: mem_total,
            temp_c: temp,
            fan_speed_pct: Some(50),
            power_watts: power,
            power_limit_watts: limit,
            clock_graphics_mhz: Some(1800),
            clock_graphics_max_mhz: Some(2100),
            clock_memory_mhz: Some(7000),
            clock_memory_max_mhz: Some(8000),
            persistence_mode: true,
            perf_state: "P0".to_string(),
            processes: procs,
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn test_filter_and_sort_processes() {
        let procs = vec![
            sample_proc(300, "python3", "python3 train.py --lr 1e-4", 4096),
            sample_proc(100, "Xorg", "/usr/lib/Xorg :0", 512),
            sample_proc(200, "blender", "blender --background", 2048),
        ];

        let desc = filter_and_sort_processes(&procs, "", GuiProcessSort::MemoryDesc);
        assert_eq!(desc.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![300, 200, 100]);

        let asc = filter_and_sort_processes(&procs, "", GuiProcessSort::MemoryAsc);
        assert_eq!(asc.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![100, 200, 300]);

        let by_pid = filter_and_sort_processes(&procs, "", GuiProcessSort::Pid);
        assert_eq!(by_pid.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![100, 200, 300]);

        let by_name = filter_and_sort_processes(&procs, "", GuiProcessSort::Name);
        assert_eq!(by_name.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![200, 300, 100]);

        let filtered_cmd = filter_and_sort_processes(&procs, "TRAIN.PY", GuiProcessSort::Pid);
        assert_eq!(filtered_cmd.len(), 1);
        assert_eq!(filtered_cmd[0].pid, 300);

        let filtered_pid = filter_and_sort_processes(&procs, "200", GuiProcessSort::Pid);
        assert_eq!(filtered_pid.len(), 1);
        assert_eq!(filtered_pid[0].name, "blender");
    }

    #[test]
    fn test_compute_fleet_summary() {
        let mut snaps = HashMap::new();
        snaps.insert(
            0,
            sample_snapshot(
                0,
                40.0,
                2_000,
                10_000,
                120.0,
                300.0,
                62,
                vec![sample_proc(10, "a", "a", 100)],
            ),
        );
        snaps.insert(
            1,
            sample_snapshot(
                1,
                80.0,
                6_000,
                10_000,
                240.0,
                300.0,
                78,
                vec![sample_proc(20, "b", "b", 200), sample_proc(30, "c", "c", 300)],
            ),
        );

        let summary = compute_fleet_summary(2, &snaps);
        assert_eq!(summary.device_count, 2);
        assert_eq!(summary.sampled_count, 2);
        assert!((summary.avg_util - 60.0).abs() < 1e-6);
        assert_eq!(summary.total_vram_used, 8_000);
        assert_eq!(summary.total_vram_cap, 20_000);
        assert!((summary.total_power_watts - 360.0).abs() < 1e-6);
        assert!((summary.total_power_limit - 600.0).abs() < 1e-6);
        assert_eq!(summary.max_temp, 78);
        assert_eq!(summary.total_procs, 3);
    }
}
