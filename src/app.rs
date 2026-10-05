use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::telemetry::{GpuHistory, GpuProcess, GpuSnapshot, TelemetryEngine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    AllGpus,
    Overview,
    Charts,
    Processes,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::AllGpus, Tab::Overview, Tab::Charts, Tab::Processes]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::AllGpus => "0 All GPUs",
            Tab::Overview => "1 Overview",
            Tab::Charts => "2 Detailed Charts",
            Tab::Processes => "3 Process Manager",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Tab::AllGpus => Tab::Overview,
            Tab::Overview => Tab::Charts,
            Tab::Charts => Tab::Processes,
            Tab::Processes => Tab::AllGpus,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::AllGpus => Tab::Processes,
            Tab::Overview => Tab::AllGpus,
            Tab::Charts => Tab::Overview,
            Tab::Processes => Tab::Charts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSort {
    MemoryDesc,
    MemoryAsc,
    Pid,
    Name,
}

impl ProcessSort {
    pub fn label(&self) -> &'static str {
        match self {
            ProcessSort::MemoryDesc => "VRAM (High to Low)",
            ProcessSort::MemoryAsc => "VRAM (Low to High)",
            ProcessSort::Pid => "PID",
            ProcessSort::Name => "Name",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ProcessSort::MemoryDesc => ProcessSort::MemoryAsc,
            ProcessSort::MemoryAsc => ProcessSort::Pid,
            ProcessSort::Pid => ProcessSort::Name,
            ProcessSort::Name => ProcessSort::MemoryDesc,
        }
    }
}

pub struct App {
    pub engine: TelemetryEngine,
    pub device_count: u32,
    pub selected_gpu: u32,
    pub monitor_all: bool,
    pub current_snapshot: Option<GpuSnapshot>,
    pub all_snapshots: HashMap<u32, GpuSnapshot>,
    pub histories: HashMap<u32, GpuHistory>,
    pub active_tab: Tab,
    pub interval: Duration,
    pub paused: bool,
    pub last_update: Instant,
    pub process_table_state: TableState,
    pub all_gpus_table_state: TableState,
    pub process_sort: ProcessSort,
    pub show_help_modal: bool,
    pub kill_confirm_proc: Option<GpuProcess>,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
}

impl App {
    pub fn new(engine: TelemetryEngine, initial_gpu: u32, interval: Duration, monitor_all: bool) -> Self {
        let count = engine.device_count().max(1);
        let selected_gpu = initial_gpu.min(count.saturating_sub(1));

        let mut histories = HashMap::new();
        for i in 0..count {
            histories.insert(i, GpuHistory::new(300));
        }

        let mut process_table_state = TableState::default();
        process_table_state.select(Some(0));

        let mut all_gpus_table_state = TableState::default();
        all_gpus_table_state.select(Some(selected_gpu as usize));

        let active_tab = if monitor_all {
            Tab::AllGpus
        } else {
            Tab::Overview
        };

        let mut app = Self {
            engine,
            device_count: count,
            selected_gpu,
            monitor_all,
            current_snapshot: None,
            all_snapshots: HashMap::new(),
            histories,
            active_tab,
            interval,
            paused: false,
            last_update: Instant::now() - interval, // Trigger immediate sample
            process_table_state,
            all_gpus_table_state,
            process_sort: ProcessSort::MemoryDesc,
            show_help_modal: false,
            kill_confirm_proc: None,
            status_message: None,
            should_quit: false,
        };

        app.refresh_telemetry();
        app
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn refresh_telemetry(&mut self) {
        if self.paused {
            return;
        }

        if self.monitor_all || self.active_tab == Tab::AllGpus {
            for i in 0..self.device_count {
                match self.engine.get_snapshot(i) {
                    Ok(snap) => {
                        let history = self
                            .histories
                            .entry(i)
                            .or_insert_with(|| GpuHistory::new(300));
                        history.record(&snap);
                        if i == self.selected_gpu {
                            self.current_snapshot = Some(snap.clone());
                        }
                        self.all_snapshots.insert(i, snap);
                    }
                    Err(e) => {
                        if i == self.selected_gpu {
                            self.set_status(format!("Error sampling GPU {}: {}", i, e));
                        }
                    }
                }
            }
            self.last_update = Instant::now();
        } else {
            match self.engine.get_snapshot(self.selected_gpu) {
                Ok(snap) => {
                    let history = self
                        .histories
                        .entry(self.selected_gpu)
                        .or_insert_with(|| GpuHistory::new(300));
                    history.record(&snap);
                    self.current_snapshot = Some(snap.clone());
                    self.all_snapshots.insert(self.selected_gpu, snap);
                    self.last_update = Instant::now();
                }
                Err(e) => {
                    self.set_status(format!("Error sampling GPU {}: {}", self.selected_gpu, e));
                }
            }
        }
    }

    pub fn get_sorted_processes(&self) -> Vec<GpuProcess> {
        let Some(snap) = &self.current_snapshot else {
            return Vec::new();
        };

        let mut procs = snap.processes.clone();
        match self.process_sort {
            ProcessSort::MemoryDesc => {
                procs.sort_by(|a, b| b.used_memory_bytes.cmp(&a.used_memory_bytes))
            }
            ProcessSort::MemoryAsc => {
                procs.sort_by(|a, b| a.used_memory_bytes.cmp(&b.used_memory_bytes))
            }
            ProcessSort::Pid => procs.sort_by_key(|p| p.pid),
            ProcessSort::Name => procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        }
        procs
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Modal dialogs capture keys first
        if self.show_help_modal {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Enter) {
                self.show_help_modal = false;
            }
            return;
        }

        if let Some(target) = self.kill_confirm_proc.clone() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.kill_process(target.pid);
                    self.kill_confirm_proc = None;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.kill_confirm_proc = None;
                    self.set_status("Kill cancelled");
                }
                _ => {}
            }
            return;
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => {
                self.show_help_modal = true;
            }
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
                if self.paused {
                    self.set_status("PAUSED - Telemetry polling frozen");
                } else {
                    self.set_status("RESUMED - Telemetry polling active");
                    self.refresh_telemetry();
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let ms = self.interval.as_millis() as u64;
                if ms > 100 {
                    self.interval = Duration::from_millis(ms.saturating_sub(100).max(100));
                    self.set_status(format!("Interval set to {} ms", self.interval.as_millis()));
                }
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                let ms = self.interval.as_millis() as u64;
                if ms < 5000 {
                    self.interval = Duration::from_millis((ms + 100).min(5000));
                    self.set_status(format!("Interval set to {} ms", self.interval.as_millis()));
                }
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.active_tab = self.active_tab.prev();
                } else {
                    self.active_tab = self.active_tab.next();
                }
            }
            KeyCode::BackTab => {
                self.active_tab = self.active_tab.prev();
            }
            KeyCode::Char('0') | KeyCode::Char('a') => self.active_tab = Tab::AllGpus,
            KeyCode::Char('1') => self.active_tab = Tab::Overview,
            KeyCode::Char('2') => self.active_tab = Tab::Charts,
            KeyCode::Char('3') => self.active_tab = Tab::Processes,
            KeyCode::Left => {
                if self.device_count > 1 {
                    self.selected_gpu = (self.selected_gpu + self.device_count - 1) % self.device_count;
                    self.all_gpus_table_state.select(Some(self.selected_gpu as usize));
                    self.current_snapshot = self.all_snapshots.get(&self.selected_gpu).cloned();
                    self.refresh_telemetry();
                }
            }
            KeyCode::Right => {
                if self.device_count > 1 {
                    self.selected_gpu = (self.selected_gpu + 1) % self.device_count;
                    self.all_gpus_table_state.select(Some(self.selected_gpu as usize));
                    self.current_snapshot = self.all_snapshots.get(&self.selected_gpu).cloned();
                    self.refresh_telemetry();
                }
            }
            KeyCode::Char('r') => {
                self.refresh_telemetry();
                self.set_status("Refreshed");
            }
            // Tab-specific controls
            _ => {
                match self.active_tab {
                    Tab::AllGpus => self.handle_all_gpus_key(key),
                    Tab::Processes => self.handle_process_key(key),
                    _ => {}
                }
            }
        }
    }

    fn handle_all_gpus_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.device_count > 1 {
                    self.selected_gpu = (self.selected_gpu + self.device_count - 1) % self.device_count;
                    self.all_gpus_table_state.select(Some(self.selected_gpu as usize));
                    self.current_snapshot = self.all_snapshots.get(&self.selected_gpu).cloned();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.device_count > 1 {
                    self.selected_gpu = (self.selected_gpu + 1) % self.device_count;
                    self.all_gpus_table_state.select(Some(self.selected_gpu as usize));
                    self.current_snapshot = self.all_snapshots.get(&self.selected_gpu).cloned();
                }
            }
            KeyCode::Enter => {
                self.active_tab = Tab::Overview;
            }
            _ => {}
        }
    }

    fn handle_process_key(&mut self, key: KeyEvent) {
        let procs = self.get_sorted_processes();
        let total = procs.len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let current = self.process_table_state.selected().unwrap_or(0);
                if total > 0 {
                    let next = if current == 0 { total - 1 } else { current - 1 };
                    self.process_table_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let current = self.process_table_state.selected().unwrap_or(0);
                if total > 0 {
                    let next = if current + 1 >= total { 0 } else { current + 1 };
                    self.process_table_state.select(Some(next));
                }
            }
            KeyCode::Home | KeyCode::Char('g') => {
                if total > 0 {
                    self.process_table_state.select(Some(0));
                }
            }
            KeyCode::End | KeyCode::Char('G') => {
                if total > 0 {
                    self.process_table_state.select(Some(total - 1));
                }
            }
            KeyCode::Char('s') => {
                self.process_sort = self.process_sort.next();
                self.set_status(format!("Sort by: {}", self.process_sort.label()));
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                if let Some(idx) = self.process_table_state.selected() {
                    if let Some(proc) = procs.get(idx) {
                        self.kill_confirm_proc = Some(proc.clone());
                    }
                }
            }
            _ => {}
        }
    }

    fn kill_process(&mut self, pid: u32) {
        // Send SIGTERM using kill command
        let res = std::process::Command::new("kill")
            .arg("-15")
            .arg(pid.to_string())
            .output();

        match res {
            Ok(output) if output.status.success() => {
                self.set_status(format!("Successfully sent SIGTERM to PID {}", pid));
                self.refresh_telemetry();
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                self.set_status(format!("Failed to kill PID {}: {}", pid, err.trim()));
            }
            Err(e) => {
                self.set_status(format!("Error terminating PID {}: {}", pid, e));
            }
        }
    }
}
