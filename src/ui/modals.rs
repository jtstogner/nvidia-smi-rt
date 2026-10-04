use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::*;
use crate::app::App;
use crate::utils::format_bytes;

/// Helper function to create a centered rect using percentages.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_help_modal(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(65, 70, area);
    f.render_widget(Clear, popup_area);

    let text = vec![
        Line::from(vec![
            Span::styled("⚡ NVIDIA-SMI-RT Help & Navigation ⚡", Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Views / Tabs:", Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  1 / 2 / 3      ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Direct switch to Overview / Charts / Processes", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(vec![
            Span::styled("  Tab / Shift-Tab", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Cycle forward / backward through tabs", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Telemetry Controls:", Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Space          ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Freeze / Pause telemetry updates", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(vec![
            Span::styled("  + / -          ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Speed up / slow down sampling rate (100ms - 5s)", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(vec![
            Span::styled("  Left / Right   ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Switch active GPU (multi-GPU systems)", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(vec![
            Span::styled("  r              ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Force immediate telemetry refresh", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Process Manager Controls (Tab 3):", Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / ↓ (j/k)    ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Navigate through running GPU tasks", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(vec![
            Span::styled("  s              ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Cycle sorting mode (VRAM High/Low, PID, Name)", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(vec![
            Span::styled("  x / Del        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Terminate selected GPU process (SIGTERM with confirm)", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("General:", Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ? / h / F1     ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Toggle this Help dialog", Style::default().fg(MUTED_GREY)),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Quit application / close popup", Style::default().fg(MUTED_GREY)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(NVIDIA_GREEN))
        .title(Span::styled(" Keyboard Shortcuts [Press Esc or Enter to close] ", Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Center);

    f.render_widget(Paragraph::new(text).block(block), popup_area);
}

pub fn render_kill_confirm_modal(f: &mut Frame, app: &App, area: Rect) {
    let Some(proc) = &app.kill_confirm_proc else {
        return;
    };

    let popup_area = centered_rect(55, 30, area);
    f.render_widget(Clear, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Are you sure you want to terminate this GPU process?", Style::default().fg(ALERT_RED).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Process: ", Style::default().fg(MUTED_GREY)),
            Span::styled(&proc.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" (PID: {})", proc.pid), Style::default().fg(ACCENT_CYAN)),
        ]),
        Line::from(vec![
            Span::styled("VRAM:    ", Style::default().fg(MUTED_GREY)),
            Span::styled(format_bytes(proc.used_memory_bytes), Style::default().fg(ALERT_AMBER)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [Y] Confirm Terminate (SIGTERM)  ", Style::default().fg(Color::Black).bg(ALERT_RED).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled("  [N / Esc] Cancel  ", Style::default().fg(Color::Black).bg(MUTED_GREY).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ALERT_RED))
        .title(Span::styled(" Terminate Process Confirmation ", Style::default().fg(ALERT_RED).add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Center);

    f.render_widget(Paragraph::new(text).alignment(Alignment::Center).block(block), popup_area);
}
