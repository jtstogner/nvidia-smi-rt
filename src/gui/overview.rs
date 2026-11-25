use eframe::egui::{self, RichText};
use egui_plot::{Corner, Legend, Line, Plot, PlotPoints};

use crate::utils::{format_bytes, format_mhz, format_temp, format_throughput_kbs, format_watts};

use super::state::{GuiApp, GuiTab};
use super::theme::{
    power_color, proc_type_color, temp_color, util_color, with_alpha, ACCENT_CYAN, ALERT_AMBER,
    ALERT_RED, NVIDIA_GREEN, PURPLE_ACCENT, SURFACE_ELEVATED, TEXT_MUTED, TEXT_PRIMARY,
};
use super::widgets::{card_frame, kpi_metric_card, pill_badge, progress_bar};

pub fn render_overview(ui: &mut egui::Ui, app: &mut GuiApp) {
    let Some(snap) = app.current_snapshot.clone() else {
        card_frame(None).show(ui, |ui| {
            ui.label(RichText::new("Waiting for GPU telemetry sample...").color(TEXT_MUTED));
        });
        return;
    };

    let history = app.histories.get(&app.selected_gpu);

    // 1. KPI Metric Cards Row
    ui.columns(4, |cols| {
        let u_color = util_color(snap.util_gpu);
        kpi_metric_card(
            &mut cols[0],
            "GPU Compute Util",
            &format!("Perf {}", snap.perf_state),
            &format!("{:.1}%", snap.util_gpu),
            &format!("Mem Ctrl: {:.0}%", snap.util_mem),
            (snap.util_gpu / 100.0) as f32,
            u_color,
            history.map(|h| &h.gpu_util.points),
            100.0,
        );

        let mem_pct = snap.mem_used_percent();
        kpi_metric_card(
            &mut cols[1],
            "VRAM Memory",
            &format!("Total {}", format_bytes(snap.mem_total_bytes)),
            &format_bytes(snap.mem_used_bytes),
            &format!("{:.1}%", mem_pct),
            (mem_pct / 100.0) as f32,
            ACCENT_CYAN,
            history.map(|h| &h.mem_util.points),
            100.0,
        );

        let p_pct = snap.power_percent();
        let p_color = power_color(snap.power_watts, snap.power_limit_watts);
        kpi_metric_card(
            &mut cols[2],
            "Power Draw",
            &format!("Cap {}", format_watts(snap.power_limit_watts)),
            &format_watts(snap.power_watts),
            &format!("{:.1}%", p_pct),
            (p_pct / 100.0) as f32,
            p_color,
            history.map(|h| &h.power_watts.points),
            snap.power_limit_watts.max(100.0),
        );

        let t_color = temp_color(snap.temp_c);
        let fan_str = snap
            .fan_speed_pct
            .map(|f| format!("Fan {}%", f))
            .unwrap_or_else(|| "Fan N/A".to_string());
        kpi_metric_card(
            &mut cols[3],
            "Thermals & Fan",
            &fan_str,
            &format_temp(snap.temp_c),
            if snap.temp_c >= 85 { "Hot" } else { "Nominal" },
            (snap.temp_c as f32 / 100.0).clamp(0.0, 1.0),
            t_color,
            history.map(|h| &h.temp_c.points),
            100.0,
        );
    });

    ui.add_space(10.0);

    // 2. Dual Realtime Interactive Plots
    let plot_height = (ui.available_height() - 250.0).clamp(210.0, 360.0);
    ui.columns(2, |cols| {
        // Left Plot: Compute & VRAM %
        card_frame(None).show(&mut cols[0], |ui| {
            ui.set_min_width(ui.available_width());
            if let Some(hist) = history {
                let (_, max_g, avg_g, cur_g) = hist.gpu_util.stats();
                let (_, _, _, cur_m) = hist.mem_util.stats();

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Compute & Memory Utilization (%)")
                            .color(TEXT_PRIMARY)
                            .strong()
                            .size(13.5),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "GPU {:.0}% (avg {:.0}% max {:.0}%)  •  VRAM {:.1}%",
                                cur_g, avg_g, max_g, cur_m
                            ))
                            .color(TEXT_MUTED)
                            .size(11.5),
                        );
                    });
                });
                ui.add_space(4.0);

                let gpu_pts: PlotPoints<'_> = hist
                    .gpu_util
                    .points
                    .iter()
                    .map(|&(x, y)| [x, y])
                    .collect();
                let mem_pts: PlotPoints<'_> = hist
                    .mem_util
                    .points
                    .iter()
                    .map(|&(x, y)| [x, y])
                    .collect();

                Plot::new("overview_util_plot")
                    .height(plot_height)
                    .include_y(0.0)
                    .include_y(100.0)
                    .legend(Legend::default().position(Corner::LeftTop))
                    .x_axis_label("Sample")
                    .y_axis_label("%")
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("GPU Compute %", gpu_pts)
                                .color(NVIDIA_GREEN)
                                .width(2.0)
                                .fill(0.0)
                                .fill_alpha(0.14),
                        );
                        plot_ui.line(
                            Line::new("VRAM %", mem_pts)
                                .color(ACCENT_CYAN)
                                .width(1.8),
                        );
                    });
            }
        });

        // Right Plot: Power (W) & Thermals (°C)
        card_frame(None).show(&mut cols[1], |ui| {
            ui.set_min_width(ui.available_width());
            if let Some(hist) = history {
                let (_, max_p, avg_p, cur_p) = hist.power_watts.stats();
                let (_, max_t, avg_t, cur_t) = hist.temp_c.stats();

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Power (W) & Thermal (°C) Dynamics")
                            .color(TEXT_PRIMARY)
                            .strong()
                            .size(13.5),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Power {:.1}W (avg {:.1} max {:.1})  •  Temp {:.0}°C (avg {:.0} max {:.0})",
                                cur_p, avg_p, max_p, cur_t, avg_t, max_t
                            ))
                            .color(TEXT_MUTED)
                            .size(11.5),
                        );
                    });
                });
                ui.add_space(4.0);

                let power_pts: PlotPoints<'_> = hist
                    .power_watts
                    .points
                    .iter()
                    .map(|&(x, y)| [x, y])
                    .collect();
                let temp_pts: PlotPoints<'_> = hist
                    .temp_c
                    .points
                    .iter()
                    .map(|&(x, y)| [x, y])
                    .collect();

                Plot::new("overview_power_temp_plot")
                    .height(plot_height)
                    .include_y(0.0)
                    .include_y(100.0)
                    .legend(Legend::default().position(Corner::LeftTop))
                    .x_axis_label("Sample")
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("Power (W)", power_pts)
                                .color(ALERT_AMBER)
                                .width(2.0)
                                .fill(0.0)
                                .fill_alpha(0.12),
                        );
                        plot_ui.line(
                            Line::new("Temp (°C)", temp_pts)
                                .color(ALERT_RED)
                                .width(1.8),
                        );
                    });
            }
        });
    });

    ui.add_space(10.0);

    // 3. Bottom Row: Hardware Details (Left) & Active Processes (Right)
    let mut open_processes_tab = false;
    ui.columns(2, |cols| {
        // Hardware & Engine Telemetry
        card_frame(None).show(&mut cols[0], |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Hardware & Engine Telemetry")
                        .color(TEXT_PRIMARY)
                        .strong()
                        .size(14.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    pill_badge(
                        ui,
                        &format!("Perf {}", snap.perf_state),
                        NVIDIA_GREEN,
                        with_alpha(NVIDIA_GREEN, 35),
                    );
                    let persist_text = if snap.persistence_mode {
                        "Persistence: On"
                    } else {
                        "Persistence: Off"
                    };
                    pill_badge(ui, persist_text, TEXT_MUTED, SURFACE_ELEVATED);
                });
            });
            ui.add_space(8.0);

            egui::Grid::new("overview_hw_grid")
                .num_columns(2)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    // Graphics Clock
                    ui.label(RichText::new("Graphics Clock").color(TEXT_MUTED));
                    let gfx_cur = snap.clock_graphics_mhz;
                    let gfx_max = snap.clock_graphics_max_mhz;
                    let gfx_frac = match (gfx_cur, gfx_max) {
                        (Some(c), Some(m)) if m > 0 => c as f32 / m as f32,
                        _ => 0.0,
                    };
                    let gfx_label = format!(
                        "{} / {}",
                        gfx_cur.map(format_mhz).unwrap_or_else(|| "N/A".to_string()),
                        gfx_max.map(format_mhz).unwrap_or_else(|| "N/A".to_string())
                    );
                    progress_bar(ui, gfx_frac, NVIDIA_GREEN, 16.0, Some(&gfx_label));
                    ui.end_row();

                    // Memory Clock
                    ui.label(RichText::new("Memory Clock").color(TEXT_MUTED));
                    let mem_cur = snap.clock_memory_mhz;
                    let mem_max = snap.clock_memory_max_mhz;
                    let mem_frac = match (mem_cur, mem_max) {
                        (Some(c), Some(m)) if m > 0 => c as f32 / m as f32,
                        _ => 0.0,
                    };
                    let mem_label = format!(
                        "{} / {}",
                        mem_cur.map(format_mhz).unwrap_or_else(|| "N/A".to_string()),
                        mem_max.map(format_mhz).unwrap_or_else(|| "N/A".to_string())
                    );
                    progress_bar(ui, mem_frac, ACCENT_CYAN, 16.0, Some(&mem_label));
                    ui.end_row();

                    // Video Encoder / Decoder
                    ui.label(RichText::new("Video Engine").color(TEXT_MUTED));
                    let enc = snap.util_encoder.unwrap_or(0.0);
                    let dec = snap.util_decoder.unwrap_or(0.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("ENC: {:.0}%", enc)).color(PURPLE_ACCENT).strong());
                        ui.separator();
                        ui.label(RichText::new(format!("DEC: {:.0}%", dec)).color(ACCENT_CYAN).strong());
                    });
                    ui.end_row();

                    // PCIe Throughput
                    ui.label(RichText::new("PCIe Traffic").color(TEXT_MUTED));
                    let tx = snap
                        .pci_tx_kbytes_per_sec
                        .map(format_throughput_kbs)
                        .unwrap_or_else(|| "N/A".to_string());
                    let rx = snap
                        .pci_rx_kbytes_per_sec
                        .map(format_throughput_kbs)
                        .unwrap_or_else(|| "N/A".to_string());
                    ui.horizontal(|ui| {
                        ui.label(format!("TX (Host→GPU): {}   |   RX (GPU→Host): {}", tx, rx));
                    });
                    ui.end_row();

                    // Bus & UUID
                    ui.label(RichText::new("Bus / UUID").color(TEXT_MUTED));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&snap.pci_bus_id).monospace().size(11.5));
                        if !snap.uuid.is_empty()
                            && ui
                                .small_button("Copy UUID")
                                .on_hover_text(&snap.uuid)
                                .clicked()
                        {
                            ui.ctx().copy_text(snap.uuid.clone());
                            app.set_status("Copied GPU UUID to clipboard");
                        }
                    });
                    ui.end_row();
                });
        });

        // Active GPU Processes (Top 6)
        card_frame(None).show(&mut cols[1], |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Active GPU Processes ({})", snap.processes.len()))
                        .color(TEXT_PRIMARY)
                        .strong()
                        .size(14.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Open Process Manager →").clicked() {
                        open_processes_tab = true;
                    }
                });
            });
            ui.add_space(6.0);

            if snap.processes.is_empty() {
                ui.label(
                    RichText::new("No active processes on this GPU.").color(TEXT_MUTED),
                );
            } else {
                egui::Grid::new("overview_top_procs")
                    .num_columns(4)
                    .spacing([14.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for h in ["PID", "Type", "Process Name", "VRAM Usage"] {
                            ui.label(RichText::new(h).color(NVIDIA_GREEN).strong().size(12.0));
                        }
                        ui.end_row();

                        for p in snap.processes.iter().take(6) {
                            let t_color = proc_type_color(p.proc_type);
                            ui.label(RichText::new(p.pid.to_string()).color(ACCENT_CYAN).monospace());
                            pill_badge(ui, p.proc_type.label(), t_color, with_alpha(t_color, 35));
                            ui.add(egui::Label::new(RichText::new(&p.name).strong()).truncate());
                            ui.label(
                                RichText::new(format_bytes(p.used_memory_bytes))
                                    .color(ALERT_AMBER),
                            );
                            ui.end_row();
                        }
                    });
            }
        });
    });

    if open_processes_tab {
        app.set_tab(GuiTab::Processes);
    }
}
