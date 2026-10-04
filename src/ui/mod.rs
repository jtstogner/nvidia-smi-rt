pub mod charts;
pub mod header;
pub mod modals;
pub mod overview;
pub mod processes;
pub mod theme;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Tab};
use header::render_header;
use modals::{render_help_modal, render_kill_confirm_modal};
use overview::render_overview;
use processes::render_processes_tab;
use theme::*;

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header + Tab bar
            Constraint::Min(12),   // Active tab contents
            Constraint::Length(1), // Bottom status bar
        ])
        .split(size);

    render_header(f, app, chunks[0]);

    match app.active_tab {
        Tab::Overview => render_overview(f, app, chunks[1]),
        Tab::Charts => charts::render_charts_tab(f, app, chunks[1]),
        Tab::Processes => render_processes_tab(f, app, chunks[1]),
    }

    render_footer(f, app, chunks[2]);

    // Modals overlay
    if app.show_help_modal {
        render_help_modal(f, size);
    } else if app.kill_confirm_proc.is_some() {
        render_kill_confirm_modal(f, app, size);
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let status_text = if let Some((msg, time)) = &app.status_message {
        if time.elapsed().as_secs() < 5 {
            Span::styled(format!("ℹ {} ", msg), Style::default().fg(ALERT_AMBER).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("Ready", Style::default().fg(MUTED_GREY))
        }
    } else {
        Span::styled("Ready", Style::default().fg(MUTED_GREY))
    };

    let footer_line = Line::from(vec![
        Span::styled(" [? / h] Help ", Style::default().fg(NVIDIA_GREEN)),
        Span::styled("| [Space] Pause ", Style::default().fg(Color::White)),
        Span::styled("| [1/2/3] Views ", Style::default().fg(Color::White)),
        Span::styled("| [+/-] Interval ", Style::default().fg(Color::White)),
        Span::styled("| [q] Quit ", Style::default().fg(Color::White)),
        Span::raw(" | "),
        status_text,
    ]);

    f.render_widget(Paragraph::new(footer_line), area);
}
