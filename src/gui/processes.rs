use eframe::egui::{self, RichText};

use crate::utils::format_bytes;

use super::state::{GuiApp, GuiProcessSort};
use super::theme::{
    proc_type_color, util_color, with_alpha, ACCENT_CYAN, ALERT_AMBER, ALERT_RED, NVIDIA_GREEN,
    TEXT_MUTED, TEXT_PRIMARY,
};
use super::widgets::{card_frame, pill_badge, progress_bar};

pub fn render_processes_tab(ui: &mut egui::Ui, app: &mut GuiApp) {
    let procs = app.filtered_sorted_processes();
    let total_vram = app
        .current_snapshot
        .as_ref()
        .map(|s| s.mem_total_bytes)
        .unwrap_or(1)
        .max(1);
    let tracked_vram: u64 = procs.iter().map(|p| p.used_memory_bytes).sum();

    // 1. Controls Bar
    card_frame(None).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("Processes: {}", procs.len()))
                    .color(TEXT_PRIMARY)
                    .strong(),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("Tracked VRAM: {}", format_bytes(tracked_vram)))
                    .color(ALERT_AMBER)
                    .strong(),
            );
            ui.separator();

            ui.label(RichText::new("Filter:").color(TEXT_MUTED));
            ui.add(
                egui::TextEdit::singleline(&mut app.process_filter)
                    .hint_text("Name, cmdline, or PID...")
                    .desired_width(220.0),
            );
            if !app.process_filter.is_empty() && ui.button("✕").on_hover_text("Clear filter").clicked() {
                app.process_filter.clear();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("↻ Refresh").clicked() {
                    app.force_refresh();
                }

                let mut sort = app.process_sort;
                egui::ComboBox::from_id_salt("proc_sort_combo")
                    .selected_text(sort.label())
                    .show_ui(ui, |ui| {
                        for &opt in GuiProcessSort::all() {
                            ui.selectable_value(&mut sort, opt, opt.label());
                        }
                    });
                if sort != app.process_sort {
                    app.process_sort = sort;
                }
                ui.label(RichText::new("Sort:").color(TEXT_MUTED));
            });
        });
    });

    ui.add_space(10.0);

    // 2. Process Table
    let table_height = (ui.available_height() - 150.0).max(200.0);
    card_frame(None).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("Active GPU Tasks")
                .color(NVIDIA_GREEN)
                .strong()
                .size(14.0),
        );
        ui.add_space(6.0);

        if procs.is_empty() {
            ui.label(
                RichText::new("No GPU processes match the current filter.")
                    .color(TEXT_MUTED),
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("proc_manager_scroll")
                .max_height(table_height)
                .show(ui, |ui| {
                    egui::Grid::new("proc_manager_table")
                        .num_columns(7)
                        .spacing([12.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for h in ["PID", "Type", "Name", "VRAM Used", "% VRAM", "Command Line", "Action"] {
                                ui.label(RichText::new(h).color(NVIDIA_GREEN).strong().size(12.0));
                            }
                            ui.end_row();

                            for proc in &procs {
                                let is_selected = app.selected_pid == Some(proc.pid);
                                let pct = (proc.used_memory_bytes as f64 / total_vram as f64) * 100.0;
                                let t_color = proc_type_color(proc.proc_type);

                                if ui
                                    .selectable_label(
                                        is_selected,
                                        RichText::new(proc.pid.to_string()).monospace(),
                                    )
                                    .clicked()
                                {
                                    app.selected_pid = Some(proc.pid);
                                }

                                pill_badge(ui, proc.proc_type.label(), t_color, with_alpha(t_color, 35));

                                if ui
                                    .selectable_label(is_selected, RichText::new(&proc.name).strong())
                                    .clicked()
                                {
                                    app.selected_pid = Some(proc.pid);
                                }

                                ui.label(
                                    RichText::new(format_bytes(proc.used_memory_bytes))
                                        .color(ALERT_AMBER),
                                );

                                ui.allocate_ui(egui::vec2(105.0, 18.0), |ui| {
                                    progress_bar(
                                        ui,
                                        (pct / 100.0) as f32,
                                        util_color(pct),
                                        15.0,
                                        Some(&format!("{:.1}%", pct)),
                                    );
                                });

                                let cmd_resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(&proc.cmdline)
                                            .color(TEXT_MUTED)
                                            .monospace(),
                                    )
                                    .truncate()
                                    .sense(egui::Sense::click()),
                                );
                                if cmd_resp.clicked() {
                                    app.selected_pid = Some(proc.pid);
                                }
                                cmd_resp.on_hover_text(&proc.cmdline);

                                let kill_btn = egui::Button::new(
                                    RichText::new("Kill").color(ALERT_RED).strong(),
                                )
                                .fill(with_alpha(ALERT_RED, 28))
                                .stroke(egui::Stroke::new(1.0, with_alpha(ALERT_RED, 130)));
                                if ui.add(kill_btn).clicked() {
                                    app.selected_pid = Some(proc.pid);
                                    app.kill_confirm_proc = Some(proc.clone());
                                }

                                ui.end_row();
                            }
                        });
                });
        }
    });

    ui.add_space(10.0);

    // 3. Process Details & Inspector Card
    let selected_proc = app
        .selected_pid
        .and_then(|pid| procs.iter().find(|p| p.pid == pid))
        .or_else(|| procs.first())
        .cloned();

    card_frame(None).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("Process Details & Inspector")
                .color(TEXT_PRIMARY)
                .strong()
                .size(13.5),
        );
        ui.add_space(4.0);

        if let Some(proc) = selected_proc {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Process:").color(TEXT_MUTED));
                ui.label(RichText::new(&proc.name).strong());
                pill_badge(
                    ui,
                    &format!("PID {}", proc.pid),
                    ACCENT_CYAN,
                    with_alpha(ACCENT_CYAN, 35),
                );
                let t_color = proc_type_color(proc.proc_type);
                pill_badge(ui, proc.proc_type.label(), t_color, with_alpha(t_color, 35));
                ui.label(
                    RichText::new(format!("VRAM: {}", format_bytes(proc.used_memory_bytes)))
                        .color(ALERT_AMBER)
                        .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let kill_btn = egui::Button::new(
                        RichText::new("Terminate Process (SIGTERM)")
                            .color(TEXT_PRIMARY)
                            .strong(),
                    )
                    .fill(with_alpha(ALERT_RED, 170));
                    if ui.add(kill_btn).clicked() {
                        app.kill_confirm_proc = Some(proc.clone());
                    }

                    if ui.button("Copy Command").clicked() {
                        ui.ctx().copy_text(proc.cmdline.clone());
                        app.set_status("Copied command line to clipboard");
                    }
                });
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Command:").color(TEXT_MUTED));
                ui.label(RichText::new(&proc.cmdline).color(ACCENT_CYAN).monospace());
            });
        } else {
            ui.label(RichText::new("No process selected.").color(TEXT_MUTED));
        }
    });
}
