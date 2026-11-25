use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::telemetry::{GpuSnapshot, TelemetryEngine};

pub enum PollerCommand {
    SetContext(egui::Context),
    SetInterval(Duration),
    SetPaused(bool),
    SetTarget { selected_gpu: u32, poll_all: bool },
    ForceRefresh,
    Shutdown,
}

pub struct SampleBatch {
    pub snapshots: Vec<(u32, GpuSnapshot)>,
    pub error: Option<String>,
}

pub struct TelemetryPoller {
    cmd_tx: mpsc::Sender<PollerCommand>,
    event_rx: mpsc::Receiver<SampleBatch>,
    pub device_count: u32,
    pub backend_name: &'static str,
    join_handle: Option<JoinHandle<()>>,
}

impl TelemetryPoller {
    pub fn start(
        force_cli: bool,
        initial_gpu: u32,
        initial_poll_all: bool,
        initial_interval: Duration,
    ) -> anyhow::Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<PollerCommand>();
        let (event_tx, event_rx) = mpsc::channel::<SampleBatch>();
        let (init_tx, init_rx) = mpsc::sync_channel::<Result<(u32, &'static str), String>>(1);

        let join_handle = thread::Builder::new()
            .name("nvml-poller".to_string())
            .spawn(move || {
                let engine = match TelemetryEngine::init(force_cli) {
                    Ok(eng) => eng,
                    Err(e) => {
                        let _ = init_tx.send(Err(e.to_string()));
                        return;
                    }
                };

                let device_count = engine.device_count().max(1);
                let backend_name = engine.backend_name();
                if init_tx.send(Ok((device_count, backend_name))).is_err() {
                    return;
                }

                let mut egui_ctx: Option<egui::Context> = None;
                let mut interval = initial_interval.max(Duration::from_millis(50));
                let mut paused = false;
                let mut selected_gpu = initial_gpu.min(device_count.saturating_sub(1));
                let mut poll_all = initial_poll_all;

                let sample_once = |engine: &TelemetryEngine,
                                   selected_gpu: u32,
                                   poll_all: bool,
                                   device_count: u32,
                                   event_tx: &mpsc::Sender<SampleBatch>,
                                   egui_ctx: &Option<egui::Context>| {
                    let mut snapshots = Vec::new();
                    let mut error = None;

                    if poll_all {
                        for i in 0..device_count {
                            match engine.get_snapshot(i) {
                                Ok(snap) => snapshots.push((i, snap)),
                                Err(e) => {
                                    if i == selected_gpu || error.is_none() {
                                        error = Some(format!("Error sampling GPU {}: {}", i, e));
                                    }
                                }
                            }
                        }
                    } else {
                        match engine.get_snapshot(selected_gpu) {
                            Ok(snap) => snapshots.push((selected_gpu, snap)),
                            Err(e) => {
                                error = Some(format!("Error sampling GPU {}: {}", selected_gpu, e));
                            }
                        }
                    }

                    let _ = event_tx.send(SampleBatch { snapshots, error });
                    if let Some(ctx) = egui_ctx {
                        ctx.request_repaint();
                    }
                };

                // Immediate initial sample on startup
                sample_once(&engine, selected_gpu, poll_all, device_count, &event_tx, &egui_ctx);
                let mut next_sample = Instant::now() + interval;

                loop {
                    let first_cmd = if paused {
                        match cmd_rx.recv() {
                            Ok(cmd) => cmd,
                            Err(_) => break,
                        }
                    } else {
                        let wait = next_sample.saturating_duration_since(Instant::now());
                        match cmd_rx.recv_timeout(wait) {
                            Ok(cmd) => cmd,
                            Err(mpsc::RecvTimeoutError::Timeout) => {
                                sample_once(
                                    &engine,
                                    selected_gpu,
                                    poll_all,
                                    device_count,
                                    &event_tx,
                                    &egui_ctx,
                                );
                                next_sample = Instant::now() + interval;
                                continue;
                            }
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    };

                    let mut should_sample_now = false;
                    let mut shutdown = false;

                    let mut apply_cmd = |cmd: PollerCommand| match cmd {
                        PollerCommand::SetContext(ctx) => {
                            egui_ctx = Some(ctx);
                        }
                        PollerCommand::SetInterval(new_interval) => {
                            interval = new_interval.max(Duration::from_millis(50));
                            next_sample = Instant::now() + interval;
                        }
                        PollerCommand::SetPaused(p) => {
                            let was_paused = paused;
                            paused = p;
                            if was_paused && !paused {
                                should_sample_now = true;
                            }
                        }
                        PollerCommand::SetTarget {
                            selected_gpu: gpu,
                            poll_all: all,
                        } => {
                            let changed = selected_gpu != gpu || poll_all != all;
                            selected_gpu = gpu.min(device_count.saturating_sub(1));
                            poll_all = all;
                            if changed && !paused {
                                should_sample_now = true;
                            }
                        }
                        PollerCommand::ForceRefresh => {
                            should_sample_now = true;
                        }
                        PollerCommand::Shutdown => {
                            shutdown = true;
                        }
                    };

                    apply_cmd(first_cmd);
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        apply_cmd(cmd);
                    }

                    if shutdown {
                        break;
                    }

                    if should_sample_now {
                        sample_once(
                            &engine,
                            selected_gpu,
                            poll_all,
                            device_count,
                            &event_tx,
                            &egui_ctx,
                        );
                        next_sample = Instant::now() + interval;
                    }
                }
            })?;

        let (device_count, backend_name) = match init_rx.recv() {
            Ok(Ok(info)) => info,
            Ok(Err(err)) => {
                let _ = join_handle.join();
                anyhow::bail!(err);
            }
            Err(_) => {
                let _ = join_handle.join();
                anyhow::bail!("Telemetry worker terminated during initialization");
            }
        };

        Ok(Self {
            cmd_tx,
            event_rx,
            device_count,
            backend_name,
            join_handle: Some(join_handle),
        })
    }

    pub fn attach_context(&self, ctx: egui::Context) {
        let _ = self.cmd_tx.send(PollerCommand::SetContext(ctx));
    }

    pub fn send(&self, cmd: PollerCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn try_recv(&self) -> Option<SampleBatch> {
        self.event_rx.try_recv().ok()
    }
}

impl Drop for TelemetryPoller {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(PollerCommand::Shutdown);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}
