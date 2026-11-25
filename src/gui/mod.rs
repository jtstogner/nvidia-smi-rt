pub mod all_gpus;
pub mod charts;
pub mod header;
pub mod modals;
pub mod overview;
pub mod poller;
pub mod processes;
pub mod state;
pub mod theme;
pub mod widgets;

use std::time::Duration;

use eframe::egui::{self, RichText, ViewportBuilder};

use poller::TelemetryPoller;
use state::{GuiApp, GuiTab};
use theme::{apply_theme, ALERT_AMBER, NVIDIA_GREEN, TEXT_MUTED};

pub struct GuiLaunchOptions {
    pub interval: Duration,
    pub initial_gpu: u32,
    pub monitor_all: bool,
    pub force_cli: bool,
    pub width: f32,
    pub height: f32,
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_samples();
        self.handle_shortcuts(ui.ctx());

        egui::Panel::top("nvidia_smi_rt_header")
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_CARD)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                header::render_header(ui, self);
            });

        egui::Panel::bottom("nvidia_smi_rt_footer")
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_CARD)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(14, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("[? / F1] Help").color(NVIDIA_GREEN).size(11.5));
                    ui.label(RichText::new("•  [Space] Pause  •  [0-3] Views  •  [+/-] Interval  •  [←/→] GPU").color(TEXT_MUTED).size(11.5));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let status = self
                            .status_message
                            .as_ref()
                            .filter(|(_, t)| t.elapsed().as_secs() < 5)
                            .map(|(m, _)| m.as_str());
                        if let Some(msg) = status {
                            ui.label(RichText::new(format!("ℹ {}", msg)).color(ALERT_AMBER).strong().size(11.5));
                        } else {
                            ui.label(
                                RichText::new(format!("Samples: {}", self.sample_count))
                                    .color(TEXT_MUTED)
                                    .size(11.5),
                            );
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_DARK)
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main_content_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_tab {
                        GuiTab::AllGpus => all_gpus::render_all_gpus(ui, self),
                        GuiTab::Overview => overview::render_overview(ui, self),
                        GuiTab::Charts => charts::render_charts_tab(ui, self),
                        GuiTab::Processes => processes::render_processes_tab(ui, self),
                    });
            });

        if self.show_help_modal {
            modals::render_help_modal(ui.ctx(), self);
        } else if self.kill_confirm_proc.is_some() {
            modals::render_kill_confirm_modal(ui.ctx(), self);
        }
    }
}

pub fn run_gui(options: GuiLaunchOptions) -> anyhow::Result<()> {
    let poller = TelemetryPoller::start(
        options.force_cli,
        options.initial_gpu,
        options.monitor_all,
        options.interval,
    )?;

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([options.width, options.height])
            .with_min_inner_size([960.0, 620.0])
            .with_title("nvidia-smi-rt — Real-Time NVIDIA GPU Telemetry"),
        ..Default::default()
    };

    eframe::run_native(
        "nvidia-smi-rt-gui",
        native_options,
        Box::new(move |cc| {
            apply_theme(&cc.egui_ctx);
            poller.attach_context(cc.egui_ctx.clone());
            Ok(Box::new(GuiApp::new(
                poller,
                options.initial_gpu,
                options.interval,
                options.monitor_all,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI event loop: {}", e))
}
