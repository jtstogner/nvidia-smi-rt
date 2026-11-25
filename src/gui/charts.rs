use eframe::egui::{self, RichText};
use egui_plot::{HLine, Line, LineStyle, Plot, PlotPoints};

use super::state::GuiApp;
use super::theme::{
    ACCENT_CYAN, ALERT_AMBER, ALERT_RED, NVIDIA_GREEN, PURPLE_ACCENT, TEXT_MUTED, TEXT_PRIMARY,
};
use super::widgets::card_frame;

pub fn render_charts_tab(ui: &mut egui::Ui, app: &mut GuiApp) {
    let Some(hist) = app.histories.get(&app.selected_gpu) else {
        card_frame(None).show(ui, |ui| {
            ui.label(RichText::new("No history recorded yet for this GPU.").color(TEXT_MUTED));
        });
        return;
    };
    let snap = app.current_snapshot.as_ref();

    // Toolbar
    ui.horizontal(|ui| {
        let count = hist.gpu_util.points.len();
        let secs = (count as f64 * app.interval.as_secs_f64()).round() as u64;
        ui.label(
            RichText::new(format!(
                "Samples: {} / {}  •  Window: ~{}s  •  Drag to pan, scroll to zoom, double-click to reset",
                count, hist.capacity, secs
            ))
            .color(TEXT_MUTED)
            .size(12.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.selectable_value(&mut app.charts_show_clock_mode, true, "Graphics Clock (MHz)");
            ui.selectable_value(&mut app.charts_show_clock_mode, false, "Thermals (°C)");
            ui.label(RichText::new("Chart 4:").color(TEXT_MUTED).size(12.0));
        });
    });

    ui.add_space(6.0);

    let plot_h = ((ui.available_height() - 92.0) * 0.5).max(180.0);

    // Top row: Compute % | VRAM GiB
    ui.columns(2, |cols| {
        // 1. GPU Core Compute Utilization (%)
        card_frame(None).show(&mut cols[0], |ui| {
            ui.set_min_width(ui.available_width());
            let (min, max, avg, cur) = hist.gpu_util.stats();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("[1] GPU Core Compute Utilization (%)")
                        .color(NVIDIA_GREEN)
                        .strong()
                        .size(13.5),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Cur {:.1}%  |  Avg {:.1}%  |  Min {:.1}%  |  Max {:.1}%",
                            cur, avg, min, max
                        ))
                        .color(TEXT_MUTED)
                        .size(11.5),
                    );
                });
            });
            ui.add_space(4.0);

            let pts: PlotPoints<'_> = hist.gpu_util.points.iter().map(|&(x, y)| [x, y]).collect();
            Plot::new("charts_gpu_util")
                .height(plot_h)
                .include_y(0.0)
                .include_y(100.0)
                .y_axis_label("%")
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new("GPU Compute %", pts)
                            .color(NVIDIA_GREEN)
                            .width(2.0)
                            .fill(0.0)
                            .fill_alpha(0.15),
                    );
                });
        });

        // 2. VRAM Allocation (GiB)
        card_frame(None).show(&mut cols[1], |ui| {
            ui.set_min_width(ui.available_width());
            let (_, max, avg, cur) = hist.mem_used_gib.stats();
            let total_gib = snap
                .map(|s| s.mem_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
                .unwrap_or(24.0)
                .max(1.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("[2] VRAM Allocation (GiB)")
                        .color(ACCENT_CYAN)
                        .strong()
                        .size(13.5),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Cur {:.2}  |  Avg {:.2}  |  Max {:.2} / {:.1} GiB",
                            cur, avg, max, total_gib
                        ))
                        .color(TEXT_MUTED)
                        .size(11.5),
                    );
                });
            });
            ui.add_space(4.0);

            let pts: PlotPoints<'_> = hist
                .mem_used_gib
                .points
                .iter()
                .map(|&(x, y)| [x, y])
                .collect();
            Plot::new("charts_vram_gib")
                .height(plot_h)
                .include_y(0.0)
                .include_y(total_gib * 1.05)
                .y_axis_label("GiB")
                .show(ui, |plot_ui| {
                    plot_ui.hline(
                        HLine::new("Total VRAM", total_gib)
                            .color(TEXT_MUTED)
                            .style(LineStyle::dashed_dense()),
                    );
                    plot_ui.line(
                        Line::new("VRAM Used (GiB)", pts)
                            .color(ACCENT_CYAN)
                            .width(2.0)
                            .fill(0.0)
                            .fill_alpha(0.15),
                    );
                });
        });
    });

    ui.add_space(10.0);

    // Bottom row: Power Draw (Watts) | Thermals / Graphics Clock
    ui.columns(2, |cols| {
        // 3. Power Draw (Watts)
        card_frame(None).show(&mut cols[0], |ui| {
            ui.set_min_width(ui.available_width());
            let (_, max, avg, cur) = hist.power_watts.stats();
            let cap = snap.map(|s| s.power_limit_watts).unwrap_or(450.0).max(100.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("[3] Power Draw (Watts)")
                        .color(ALERT_AMBER)
                        .strong()
                        .size(13.5),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Cur {:.1}W  |  Avg {:.1}W  |  Max {:.1}W (Cap {:.0}W)",
                            cur, avg, max, cap
                        ))
                        .color(TEXT_MUTED)
                        .size(11.5),
                    );
                });
            });
            ui.add_space(4.0);

            let pts: PlotPoints<'_> = hist
                .power_watts
                .points
                .iter()
                .map(|&(x, y)| [x, y])
                .collect();
            Plot::new("charts_power_w")
                .height(plot_h)
                .include_y(0.0)
                .include_y(cap * 1.05)
                .y_axis_label("Watts")
                .show(ui, |plot_ui| {
                    plot_ui.hline(
                        HLine::new("Power Limit", cap)
                            .color(ALERT_RED)
                            .style(LineStyle::dashed_dense()),
                    );
                    plot_ui.line(
                        Line::new("Power Draw (W)", pts)
                            .color(ALERT_AMBER)
                            .width(2.0)
                            .fill(0.0)
                            .fill_alpha(0.15),
                    );
                });
        });

        // 4. Thermals (°C) or Graphics Clock (MHz)
        card_frame(None).show(&mut cols[1], |ui| {
            ui.set_min_width(ui.available_width());
            let (_, max_t, avg_t, cur_t) = hist.temp_c.stats();
            let (_, max_c, avg_c, cur_c) = hist.clock_graphics.stats();

            ui.horizontal(|ui| {
                let title = if app.charts_show_clock_mode {
                    "[4] Graphics Engine Clock (MHz)"
                } else {
                    "[4] GPU Core Thermals (°C)"
                };
                let title_color = if app.charts_show_clock_mode {
                    PURPLE_ACCENT
                } else {
                    ALERT_RED
                };
                ui.label(RichText::new(title).color(title_color).strong().size(13.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let summary = if app.charts_show_clock_mode {
                        format!(
                            "Clock {:.0} MHz (avg {:.0}, max {:.0})  •  Temp {:.0}°C",
                            cur_c, avg_c, max_c, cur_t
                        )
                    } else {
                        format!(
                            "Temp {:.0}°C (avg {:.0}°C, max {:.0}°C)  •  Clock {:.0} MHz",
                            cur_t, avg_t, max_t, cur_c
                        )
                    };
                    ui.label(RichText::new(summary).color(TEXT_MUTED).size(11.5));
                });
            });
            ui.add_space(4.0);

            if app.charts_show_clock_mode {
                let max_clk = snap
                    .and_then(|s| s.clock_graphics_max_mhz)
                    .map(|m| m as f64)
                    .unwrap_or(2500.0)
                    .max(500.0);
                let pts: PlotPoints<'_> = hist
                    .clock_graphics
                    .points
                    .iter()
                    .map(|&(x, y)| [x, y])
                    .collect();

                Plot::new("charts_clock_mhz")
                    .height(plot_h)
                    .include_y(0.0)
                    .include_y(max_clk * 1.05)
                    .y_axis_label("MHz")
                    .show(ui, |plot_ui| {
                        plot_ui.hline(
                            HLine::new("Boost Max", max_clk)
                                .color(TEXT_MUTED)
                                .style(LineStyle::dashed_dense()),
                        );
                        plot_ui.line(
                            Line::new("Graphics Clock (MHz)", pts)
                                .color(PURPLE_ACCENT)
                                .width(2.0)
                                .fill(0.0)
                                .fill_alpha(0.15),
                        );
                    });
            } else {
                let pts: PlotPoints<'_> = hist.temp_c.points.iter().map(|&(x, y)| [x, y]).collect();
                Plot::new("charts_temp_c")
                    .height(plot_h)
                    .include_y(0.0)
                    .include_y(105.0)
                    .y_axis_label("°C")
                    .show(ui, |plot_ui| {
                        plot_ui.hline(
                            HLine::new("85°C Thermal Threshold", 85.0)
                                .color(ALERT_AMBER)
                                .style(LineStyle::dashed_dense()),
                        );
                        plot_ui.line(
                            Line::new("GPU Temp (°C)", pts)
                                .color(ALERT_RED)
                                .width(2.0)
                                .fill(0.0)
                                .fill_alpha(0.15),
                        );
                    });
            }
        });
    });

    let _ = TEXT_PRIMARY;
}
