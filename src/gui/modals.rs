use eframe::egui::{self, Align2, RichText};

use crate::utils::format_bytes;

use super::state::GuiApp;
use super::theme::{with_alpha, ACCENT_CYAN, ALERT_AMBER, ALERT_RED, NVIDIA_GREEN, TEXT_MUTED, TEXT_PRIMARY};

pub fn render_help_modal(ctx: &egui::Context, app: &mut GuiApp) {
    let mut open = app.show_help_modal;
    egui::Window::new("Keyboard Shortcuts & Controls")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label(
                RichText::new("⚡ NVIDIA-SMI-RT GUI Reference")
                    .color(NVIDIA_GREEN)
                    .strong()
                    .size(15.0),
            );
            ui.add_space(8.0);

            egui::Grid::new("help_shortcuts_grid")
                .num_columns(2)
                .spacing([20.0, 6.0])
                .show(ui, |ui| {
                    let rows = [
                        ("0 / a", "All GPUs Fleet View"),
                        ("1 / 2 / 3", "Switch to Overview / Detailed Charts / Process Manager"),
                        ("Tab / Shift+Tab", "Cycle forward / backward through views"),
                        ("Space", "Pause / resume telemetry polling"),
                        ("+ / -", "Speed up / slow down polling interval (100ms - 5s)"),
                        ("Left / Right", "Switch active GPU (multi-GPU workstations)"),
                        ("r", "Trigger immediate telemetry refresh"),
                        ("s", "Cycle process sort mode in Process Manager"),
                        ("Plot Drag / Scroll", "Pan and zoom interactive time-series plots"),
                        ("Plot Double-Click", "Reset plot zoom & auto-bounds"),
                        ("? / h / F1", "Toggle this Help dialog"),
                        ("Esc / Enter", "Close active dialog"),
                    ];
                    for (k, desc) in rows {
                        ui.label(RichText::new(k).color(ACCENT_CYAN).monospace().strong());
                        ui.label(RichText::new(desc).color(TEXT_PRIMARY));
                        ui.end_row();
                    }
                });

            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    app.show_help_modal = false;
                }
            });
        });

    if !open {
        app.show_help_modal = false;
    }
}

pub fn render_kill_confirm_modal(ctx: &egui::Context, app: &mut GuiApp) {
    let Some(target) = app.kill_confirm_proc.clone() else {
        return;
    };

    let mut still_open = true;
    egui::Window::new("Terminate Process Confirmation")
        .open(&mut still_open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(480.0)
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Are you sure you want to send SIGTERM to this GPU process?")
                    .color(ALERT_RED)
                    .strong()
                    .size(14.0),
            );
            ui.add_space(8.0);

            egui::Grid::new("kill_confirm_grid")
                .num_columns(2)
                .spacing([14.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Process:").color(TEXT_MUTED));
                    ui.label(RichText::new(format!("{} (PID {})", target.name, target.pid)).strong());
                    ui.end_row();

                    ui.label(RichText::new("VRAM:").color(TEXT_MUTED));
                    ui.label(RichText::new(format_bytes(target.used_memory_bytes)).color(ALERT_AMBER));
                    ui.end_row();

                    ui.label(RichText::new("Command:").color(TEXT_MUTED));
                    ui.label(RichText::new(&target.cmdline).color(ACCENT_CYAN).monospace());
                    ui.end_row();
                });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let confirm_btn = egui::Button::new(
                    RichText::new("Confirm Terminate (SIGTERM) [Y]")
                        .color(TEXT_PRIMARY)
                        .strong(),
                )
                .fill(with_alpha(ALERT_RED, 190));

                if ui.add(confirm_btn).clicked() {
                    app.terminate_selected(target.pid);
                    app.kill_confirm_proc = None;
                }

                if ui.button("Cancel [N / Esc]").clicked() {
                    app.kill_confirm_proc = None;
                    app.set_status("Kill cancelled");
                }
            });
        });

    if !still_open {
        app.kill_confirm_proc = None;
    }
}
