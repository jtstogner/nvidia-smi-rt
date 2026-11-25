use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

use crate::telemetry::ProcessType;

pub const BG_DARK: Color32 = Color32::from_rgb(11, 15, 23);
pub const SURFACE_CARD: Color32 = Color32::from_rgb(17, 24, 39);
pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(22, 32, 50);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(31, 41, 55);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(51, 65, 85);

pub const NVIDIA_GREEN: Color32 = Color32::from_rgb(118, 185, 0);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
pub const ALERT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);
pub const ALERT_RED: Color32 = Color32::from_rgb(239, 68, 68);
pub const PURPLE_ACCENT: Color32 = Color32::from_rgb(168, 85, 247);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(248, 250, 252);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn util_color(pct: f64) -> Color32 {
    if pct < 50.0 {
        NVIDIA_GREEN
    } else if pct < 80.0 {
        ALERT_AMBER
    } else {
        ALERT_RED
    }
}

pub fn temp_color(c: u32) -> Color32 {
    if c < 60 {
        ACCENT_CYAN
    } else if c < 75 {
        NVIDIA_GREEN
    } else if c < 85 {
        ALERT_AMBER
    } else {
        ALERT_RED
    }
}

pub fn power_color(watts: f64, limit: f64) -> Color32 {
    if limit <= 0.0 {
        return NVIDIA_GREEN;
    }
    let pct = (watts / limit) * 100.0;
    if pct < 60.0 {
        NVIDIA_GREEN
    } else if pct < 85.0 {
        ALERT_AMBER
    } else {
        ALERT_RED
    }
}

pub fn proc_type_color(proc_type: ProcessType) -> Color32 {
    match proc_type {
        ProcessType::Compute => NVIDIA_GREEN,
        ProcessType::Graphics => ACCENT_CYAN,
        ProcessType::ComputeAndGraphics => PURPLE_ACCENT,
    }
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    let mut visuals = Visuals::dark();

    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_DARK;
    visuals.window_fill = SURFACE_CARD;
    visuals.extreme_bg_color = Color32::from_rgb(7, 10, 16);
    visuals.faint_bg_color = SURFACE_ELEVATED;
    visuals.window_stroke = Stroke::new(1.0, BORDER_STRONG);
    visuals.window_corner_radius = CornerRadius::same(10);

    visuals.selection.bg_fill = with_alpha(NVIDIA_GREEN, 55);
    visuals.selection.stroke = Stroke::new(1.0, NVIDIA_GREEN);

    visuals.widgets.noninteractive.bg_fill = SURFACE_CARD;
    visuals.widgets.noninteractive.weak_bg_fill = SURFACE_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = SURFACE_ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = SURFACE_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(30, 41, 59);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(30, 41, 59);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, NVIDIA_GREEN);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = with_alpha(NVIDIA_GREEN, 45);
    visuals.widgets.active.weak_bg_fill = with_alpha(NVIDIA_GREEN, 45);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, NVIDIA_GREEN);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(14);
    ctx.set_global_style(style);
}
