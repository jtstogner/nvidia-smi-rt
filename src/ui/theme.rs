use ratatui::style::{Color, Modifier, Style};

pub const NVIDIA_GREEN: Color = Color::Rgb(118, 185, 0);
pub const ACCENT_CYAN: Color = Color::Rgb(56, 189, 248);
pub const ALERT_AMBER: Color = Color::Rgb(251, 191, 36);
pub const ALERT_RED: Color = Color::Rgb(239, 68, 68);
pub const MUTED_GREY: Color = Color::Rgb(148, 163, 184);
pub const DARK_BORDER: Color = Color::Rgb(51, 65, 85);
pub const PURPLE_ACCENT: Color = Color::Rgb(168, 85, 247);

#[allow(dead_code)]
pub fn style_bold_green() -> Style {
    Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)
}

#[allow(dead_code)]
pub fn style_muted() -> Style {
    Style::default().fg(MUTED_GREY)
}

pub fn util_color(pct: f64) -> Color {
    if pct < 50.0 {
        NVIDIA_GREEN
    } else if pct < 80.0 {
        ALERT_AMBER
    } else {
        ALERT_RED
    }
}

pub fn temp_color(c: u32) -> Color {
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

pub fn power_color(watts: f64, limit: f64) -> Color {
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
