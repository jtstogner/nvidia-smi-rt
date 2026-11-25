use chrono::Local;
use eframe::egui::{self, RichText};

use super::state::{GuiApp, GuiTab};
use super::theme::{
    with_alpha, ACCENT_CYAN, ALERT_AMBER, BORDER_SUBTLE, NVIDIA_GREEN, SURFACE_ELEVATED,
    TEXT_MUTED, TEXT_PRIMARY,
};
use super::widgets::pill_badge;

const PRESET_INTERVALS_MS: &[u64] = &[100, 250, 500, 1000, 2000, 5000];

pub fn render_header(ui: &mut egui::Ui, app: &mut GuiApp) {
    ui.vertical(|ui| {
        // Row 1: Brand, GPU selector, Hardware metadata, Live controls
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("⚡ NVIDIA-SMI-RT")
                    .color(NVIDIA_GREEN)
                    .strong()
                    .size(17.0),
            );
            ui.separator();

            if app.device_count > 1 {
                let current_label = app
                    .all_snapshots
                    .get(&app.selected_gpu)
                    .map(|s| format!("GPU {}: {}", app.selected_gpu, s.name))
                    .unwrap_or_else(|| format!("GPU {}", app.selected_gpu));

                let mut chosen = app.selected_gpu;
                egui::ComboBox::from_id_salt("header_gpu_select")
                    .selected_text(RichText::new(current_label).strong())
                    .show_ui(ui, |ui| {
                        for idx in 0..app.device_count {
                            let label = app
                                .all_snapshots
                                .get(&idx)
                                .map(|s| format!("GPU {}: {}", idx, s.name))
                                .unwrap_or_else(|| format!("GPU {}", idx));
                            ui.selectable_value(&mut chosen, idx, label);
                        }
                    });
                if chosen != app.selected_gpu {
                    app.select_gpu(chosen);
                }
            } else if let Some(snap) = &app.current_snapshot {
                pill_badge(
                    ui,
                    &format!("GPU 0: {}", snap.name),
                    TEXT_PRIMARY,
                    SURFACE_ELEVATED,
                );
            } else {
                pill_badge(ui, "Detecting GPU...", TEXT_MUTED, SURFACE_ELEVATED);
            }

            if let Some(snap) = &app.current_snapshot {
                let pci_gen = snap
                    .pci_link_gen
                    .map(|g| format!("Gen{}", g))
                    .unwrap_or_default();
                let pci_w = snap
                    .pci_link_width
                    .map(|w| format!("x{}", w))
                    .unwrap_or_default();
                let pci_str = if pci_gen.is_empty() && pci_w.is_empty() {
                    snap.pci_bus_id.clone()
                } else {
                    format!("PCIe {} {}", pci_gen, pci_w)
                };

                ui.label(
                    RichText::new(format!(
                        "Driver {}  •  CUDA {}  •  ",
                        snap.driver_version, snap.cuda_version
                    ))
                    .color(TEXT_MUTED)
                    .size(12.0),
                );
                ui.label(RichText::new(pci_str).color(ACCENT_CYAN).size(12.0));
            }

            // Right-aligned live controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let now_str = Local::now().format("%H:%M:%S").to_string();
                ui.label(RichText::new(now_str).color(TEXT_PRIMARY).monospace().size(13.0));

                if ui.button("?").on_hover_text("Keyboard shortcuts (? / F1)").clicked() {
                    app.show_help_modal = true;
                }

                if ui.button("↻").on_hover_text("Refresh now (r)").clicked() {
                    app.force_refresh();
                }

                // Interval controls: [+] [ComboBox] [-]
                let current_ms = app.interval.as_millis() as u64;
                if ui.button("+").on_hover_text("Increase interval (+100ms)").clicked() {
                    app.set_interval(std::time::Duration::from_millis((current_ms + 100).min(5000)));
                }

                let mut chosen_ms = current_ms;
                egui::ComboBox::from_id_salt("header_interval_select")
                    .width(78.0)
                    .selected_text(format!("{} ms", current_ms))
                    .show_ui(ui, |ui| {
                        for &ms in PRESET_INTERVALS_MS {
                            ui.selectable_value(&mut chosen_ms, ms, format!("{} ms", ms));
                        }
                    });
                if chosen_ms != current_ms {
                    app.set_interval(std::time::Duration::from_millis(chosen_ms));
                }

                if ui.button("-").on_hover_text("Decrease interval (-100ms)").clicked() {
                    app.set_interval(std::time::Duration::from_millis(current_ms.saturating_sub(100).max(100)));
                }

                let (pause_label, pause_fg, pause_bg) = if app.paused {
                    ("⏸ PAUSED", ALERT_AMBER, with_alpha(ALERT_AMBER, 38))
                } else {
                    ("● LIVE", NVIDIA_GREEN, with_alpha(NVIDIA_GREEN, 38))
                };
                let pause_btn = egui::Button::new(RichText::new(pause_label).color(pause_fg).strong())
                    .fill(pause_bg)
                    .stroke(egui::Stroke::new(1.0, with_alpha(pause_fg, 140)));
                if ui
                    .add(pause_btn)
                    .on_hover_text("Pause / resume polling (Space)")
                    .clicked()
                {
                    app.set_paused(!app.paused);
                }
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(2.0);

        // Row 2: Tab strip + backend indicator
        ui.horizontal(|ui| {
            for &tab in GuiTab::all() {
                let is_active = app.active_tab == tab;
                let text = RichText::new(tab.title())
                    .color(if is_active { TEXT_PRIMARY } else { TEXT_MUTED })
                    .strong();
                let btn = egui::Button::new(text)
                    .fill(if is_active {
                        with_alpha(NVIDIA_GREEN, 42)
                    } else {
                        SURFACE_ELEVATED
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if is_active { NVIDIA_GREEN } else { BORDER_SUBTLE },
                    ));
                if ui.add(btn).clicked() {
                    app.set_tab(tab);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Backend: {}", app.poller.backend_name))
                        .color(TEXT_MUTED)
                        .size(11.5),
                );
            });
        });
    });
}
