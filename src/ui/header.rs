use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use super::theme::*;
use crate::app::{App, Tab};

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    let top_area = chunks[0];
    let tab_area = chunks[1];

    let now_str = Local::now().format("%H:%M:%S").to_string();

    let snap = app.current_snapshot.as_ref();

    let gpu_name = snap
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "Detecting...".to_string());

    let driver = snap
        .map(|s| s.driver_version.clone())
        .unwrap_or_else(|| "N/A".to_string());

    let cuda = snap
        .map(|s| s.cuda_version.clone())
        .unwrap_or_else(|| "N/A".to_string());

    let pci_info = snap
        .map(|s| {
            let pci_gen = s.pci_link_gen.map(|g| format!("Gen{}", g)).unwrap_or_default();
            let width = s.pci_link_width.map(|w| format!("x{}", w)).unwrap_or_default();
            if pci_gen.is_empty() && width.is_empty() {
                s.pci_bus_id.clone()
            } else {
                format!("PCIe {} {}", pci_gen, width)
            }
        })
        .unwrap_or_else(|| "N/A".to_string());

    let (status_badge, status_style) = if app.paused {
        (" ⏸ PAUSED ", Style::default().fg(Color::Black).bg(ALERT_AMBER).add_modifier(Modifier::BOLD))
    } else {
        (" ● LIVE ", Style::default().fg(Color::Black).bg(NVIDIA_GREEN).add_modifier(Modifier::BOLD))
    };

    let title_line = Line::from(vec![
        Span::styled("⚡ NVIDIA-SMI-RT ", Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(format!("[GPU {}/{}: {}] ", app.selected_gpu, app.device_count, gpu_name), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!("Driver: {} ", driver), Style::default().fg(MUTED_GREY)),
        Span::styled(format!("CUDA: {} ", cuda), Style::default().fg(MUTED_GREY)),
        Span::styled(format!("| {} ", pci_info), Style::default().fg(ACCENT_CYAN)),
        Span::styled(format!("| {}ms ", app.interval.as_millis()), Style::default().fg(MUTED_GREY)),
        Span::styled(status_badge, status_style),
        Span::raw(" "),
        Span::styled(now_str, Style::default().fg(Color::White)),
    ]);

    let top_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DARK_BORDER));

    f.render_widget(Paragraph::new(title_line).block(top_block), top_area);

    // Tab navigation bar
    let tab_titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let title = t.title();
            if *t == app.active_tab {
                Line::from(vec![
                    Span::styled(" [ ", Style::default().fg(NVIDIA_GREEN)),
                    Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(" ] ", Style::default().fg(NVIDIA_GREEN)),
                ])
            } else {
                Line::from(vec![Span::styled(format!("  {}  ", title), Style::default().fg(MUTED_GREY))])
            }
        })
        .collect();

    let tab_index = match app.active_tab {
        Tab::Overview => 0,
        Tab::Charts => 1,
        Tab::Processes => 2,
    };

    let tabs_widget = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DARK_BORDER))
                .title(Span::styled(" Views [1/2/3 or Tab] ", Style::default().fg(MUTED_GREY))),
        )
        .select(tab_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(NVIDIA_GREEN));

    f.render_widget(tabs_widget, tab_area);
}
