use eframe::egui::{self, RichText};

use crate::telemetry::GpuProcess;
use crate::utils::{format_bytes, format_mhz, format_temp, format_watts};

use super::state::{GuiApp, GuiTab};
use super::theme::{
    power_color, proc_type_color, temp_color, util_color, with_alpha, ACCENT_CYAN, NVIDIA_GREEN,
    SURFACE_ELEVATED, TEXT_MUTED, TEXT_PRIMARY,
};
use super::widgets::{card_frame, pill_badge, progress_bar};

pub fn render_all_gpus(ui: &mut egui::Ui, app: &mut GuiApp) {
    let summary = app.fleet_summary();
    let vram_pct = if summary.total_vram_cap > 0 {
        (summary.total_vram_used as f64 / summary.total_vram_cap as f64) * 100.0
    } else {
        0.0
    };
    let power_pct = if summary.total_power_limit > 0.0 {
        (summary.total_power_watts / summary.total_power_limit) * 100.0
    } else {
        0.0
    };

    // 1. Fleet Summary Cards
    ui.columns(4, |cols| {
        let c1 = util_color(summary.avg_util);
        card_frame(Some(with_alpha(c1, 110))).show(&mut cols[0], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Fleet Devices & Avg Load").color(c1).strong().size(12.5));
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("{} Devices  •  {:.1}%", summary.device_count, summary.avg_util))
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(18.0),
            );
            ui.add_space(6.0);
            progress_bar(ui, (summary.avg_util / 100.0) as f32, c1, 8.0, None);
            ui.add_space(2.0);
            ui.label(RichText::new("Cluster Compute Average").color(TEXT_MUTED).size(11.5));
        });

        card_frame(Some(with_alpha(ACCENT_CYAN, 110))).show(&mut cols[1], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Total VRAM Allocation").color(ACCENT_CYAN).strong().size(12.5));
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} / {}",
                    format_bytes(summary.total_vram_used),
                    format_bytes(summary.total_vram_cap)
                ))
                .color(TEXT_PRIMARY)
                .strong()
                .size(17.0),
            );
            ui.add_space(6.0);
            progress_bar(ui, (vram_pct / 100.0) as f32, ACCENT_CYAN, 8.0, None);
            ui.add_space(2.0);
            ui.label(RichText::new(format!("{:.1}% Fleet Memory", vram_pct)).color(TEXT_MUTED).size(11.5));
        });

        let c3 = power_color(summary.total_power_watts, summary.total_power_limit);
        card_frame(Some(with_alpha(c3, 110))).show(&mut cols[2], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Total Power Draw").color(c3).strong().size(12.5));
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} / {}",
                    format_watts(summary.total_power_watts),
                    format_watts(summary.total_power_limit)
                ))
                .color(TEXT_PRIMARY)
                .strong()
                .size(17.0),
            );
            ui.add_space(6.0);
            progress_bar(ui, (power_pct / 100.0) as f32, c3, 8.0, None);
            ui.add_space(2.0);
            ui.label(RichText::new(format!("{:.1}% Power Envelope", power_pct)).color(TEXT_MUTED).size(11.5));
        });

        let c4 = temp_color(summary.max_temp);
        card_frame(Some(with_alpha(c4, 110))).show(&mut cols[3], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Thermals & Tasks").color(c4).strong().size(12.5));
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "Peak {}  •  {} Tasks",
                    format_temp(summary.max_temp),
                    summary.total_procs
                ))
                .color(TEXT_PRIMARY)
                .strong()
                .size(17.0),
            );
            ui.add_space(6.0);
            progress_bar(ui, (summary.max_temp as f32 / 100.0).clamp(0.0, 1.0), c4, 8.0, None);
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!("Monitored: {}/{} GPUs", summary.sampled_count, summary.device_count))
                    .color(TEXT_MUTED)
                    .size(11.5),
            );
        });
    });

    ui.add_space(10.0);

    // 2. Monitored GPU Devices Table
    let mut inspect_gpu: Option<u32> = None;
    let mut select_only_gpu: Option<u32> = None;

    card_frame(None).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Monitored GPU Devices ({})", app.device_count))
                    .color(NVIDIA_GREEN)
                    .strong()
                    .size(14.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("Select a GPU or click Inspect to open Overview")
                        .color(TEXT_MUTED)
                        .size(11.5),
                );
            });
        });
        ui.add_space(6.0);

        egui::ScrollArea::horizontal()
            .id_salt("fleet_gpu_table_scroll")
            .show(ui, |ui| {
                egui::Grid::new("fleet_gpu_table")
                    .num_columns(10)
                    .spacing([14.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for h in [
                            "GPU",
                            "Device Name",
                            "Compute Util",
                            "VRAM Allocation",
                            "Power",
                            "Temp / Fan",
                            "Clock",
                            "PCIe",
                            "Tasks",
                            "Action",
                        ] {
                            ui.label(RichText::new(h).color(ACCENT_CYAN).strong().size(12.0));
                        }
                        ui.end_row();

                        for i in 0..app.device_count {
                            let is_selected = i == app.selected_gpu;
                            let gpu_btn = egui::Button::new(
                                RichText::new(if is_selected {
                                    format!("▶ GPU {}", i)
                                } else {
                                    format!("GPU {}", i)
                                })
                                .color(if is_selected { NVIDIA_GREEN } else { TEXT_PRIMARY })
                                .strong(),
                            )
                            .fill(if is_selected {
                                with_alpha(NVIDIA_GREEN, 35)
                            } else {
                                SURFACE_ELEVATED
                            });
                            if ui.add(gpu_btn).clicked() {
                                select_only_gpu = Some(i);
                            }

                            if let Some(snap) = app.all_snapshots.get(&i) {
                                ui.label(
                                    RichText::new(&snap.name)
                                        .color(if is_selected { NVIDIA_GREEN } else { TEXT_PRIMARY })
                                        .strong(),
                                );

                                let u_color = util_color(snap.util_gpu);
                                ui.allocate_ui(egui::vec2(130.0, 20.0), |ui| {
                                    progress_bar(
                                        ui,
                                        (snap.util_gpu / 100.0) as f32,
                                        u_color,
                                        16.0,
                                        Some(&format!("{:.1}%", snap.util_gpu)),
                                    );
                                });

                                let mem_pct = snap.mem_used_percent();
                                ui.allocate_ui(egui::vec2(190.0, 20.0), |ui| {
                                    progress_bar(
                                        ui,
                                        (mem_pct / 100.0) as f32,
                                        ACCENT_CYAN,
                                        16.0,
                                        Some(&format!(
                                            "{:.1}% ({} / {})",
                                            mem_pct,
                                            format_bytes(snap.mem_used_bytes),
                                            format_bytes(snap.mem_total_bytes)
                                        )),
                                    );
                                });

                                let p_color = power_color(snap.power_watts, snap.power_limit_watts);
                                ui.label(
                                    RichText::new(format!(
                                        "{} / {}",
                                        format_watts(snap.power_watts),
                                        format_watts(snap.power_limit_watts)
                                    ))
                                    .color(p_color),
                                );

                                let t_color = temp_color(snap.temp_c);
                                let fan_str = snap
                                    .fan_speed_pct
                                    .map(|f| format!("{}%", f))
                                    .unwrap_or_else(|| "N/A".to_string());
                                ui.label(
                                    RichText::new(format!("{} | Fan {}", format_temp(snap.temp_c), fan_str))
                                        .color(t_color),
                                );

                                let clk_str = snap
                                    .clock_graphics_mhz
                                    .map(format_mhz)
                                    .unwrap_or_else(|| "N/A".to_string());
                                ui.label(RichText::new(clk_str).color(TEXT_MUTED));

                                let pci_gen = snap.pci_link_gen.map(|g| format!("Gen{}", g)).unwrap_or_default();
                                let pci_w = snap.pci_link_width.map(|w| format!("x{}", w)).unwrap_or_default();
                                let pci_str = if pci_gen.is_empty() && pci_w.is_empty() {
                                    "N/A".to_string()
                                } else {
                                    format!("{} {}", pci_gen, pci_w)
                                };
                                ui.label(RichText::new(pci_str).color(TEXT_MUTED));

                                ui.label(format!("{} tasks", snap.processes.len()));

                                if ui.button("Inspect →").clicked() {
                                    inspect_gpu = Some(i);
                                }
                            } else {
                                ui.label(RichText::new("Polling telemetry...").color(TEXT_MUTED));
                                for _ in 0..8 {
                                    ui.label("-");
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
    });

    if let Some(gpu) = inspect_gpu {
        app.select_gpu(gpu);
        app.set_tab(GuiTab::Overview);
        return;
    } else if let Some(gpu) = select_only_gpu {
        app.select_gpu(gpu);
    }

    ui.add_space(10.0);

    // 3. Fleet-wide Active Processes
    let mut all_procs: Vec<(u32, &GpuProcess, u64)> = Vec::new();
    for i in 0..app.device_count {
        if let Some(snap) = app.all_snapshots.get(&i) {
            for p in &snap.processes {
                all_procs.push((i, p, snap.mem_total_bytes));
            }
        }
    }
    all_procs.sort_by(|a, b| b.1.used_memory_bytes.cmp(&a.1.used_memory_bytes));

    card_frame(None).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new(format!("Active Processes Across All GPUs ({})", all_procs.len()))
                .color(ACCENT_CYAN)
                .strong()
                .size(14.0),
        );
        ui.add_space(6.0);

        if all_procs.is_empty() {
            ui.label(
                RichText::new("No active compute/graphics processes detected across all GPUs.")
                    .color(TEXT_MUTED),
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("fleet_procs_scroll")
                .max_height(ui.available_height().max(180.0))
                .show(ui, |ui| {
                    egui::Grid::new("fleet_procs_table")
                        .num_columns(7)
                        .spacing([14.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for h in ["GPU", "PID", "Type", "VRAM", "% GPU", "Process Name", "Command Line"] {
                                ui.label(RichText::new(h).color(ACCENT_CYAN).strong().size(12.0));
                            }
                            ui.end_row();

                            for (gpu_idx, p, total_mem) in all_procs {
                                let pct = if total_mem > 0 {
                                    (p.used_memory_bytes as f64 / total_mem as f64) * 100.0
                                } else {
                                    0.0
                                };
                                let t_color = proc_type_color(p.proc_type);
                                pill_badge(
                                    ui,
                                    &format!("GPU {}", gpu_idx),
                                    TEXT_PRIMARY,
                                    SURFACE_ELEVATED,
                                );
                                ui.label(RichText::new(p.pid.to_string()).monospace());
                                pill_badge(ui, p.proc_type.label(), t_color, with_alpha(t_color, 35));
                                ui.label(
                                    RichText::new(format_bytes(p.used_memory_bytes))
                                        .color(ACCENT_CYAN),
                                );
                                ui.label(format!("{:.1}%", pct));
                                ui.label(RichText::new(&p.name).strong());
                                let cmd = if p.cmdline.is_empty() { &p.name } else { &p.cmdline };
                                ui.add(
                                    egui::Label::new(RichText::new(cmd).color(TEXT_MUTED).monospace())
                                        .truncate(),
                                )
                                .on_hover_text(cmd);
                                ui.end_row();
                            }
                        });
                });
        }
    });
}
