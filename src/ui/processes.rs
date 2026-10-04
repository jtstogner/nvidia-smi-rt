use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use super::theme::*;
use crate::app::App;
use crate::utils::format_bytes;

pub fn render_processes_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Controls bar
            Constraint::Min(8),    // Process Table
            Constraint::Length(6), // Selected Process Inspector
        ])
        .split(area);

    let procs = app.get_sorted_processes();
    let total_vram = app
        .current_snapshot
        .as_ref()
        .map(|s| s.mem_total_bytes)
        .unwrap_or(1);

    // 1. Controls bar
    let sum_proc_vram: u64 = procs.iter().map(|p| p.used_memory_bytes).sum();
    let controls_line = Line::from(vec![
        Span::styled(format!(" Total Processes: {} ", procs.len()), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!("| Tracked VRAM: {} ", format_bytes(sum_proc_vram)), Style::default().fg(ALERT_AMBER)),
        Span::styled(format!("| Sort: {} [s] ", app.process_sort.label()), Style::default().fg(ACCENT_CYAN)),
        Span::styled("| [↑/↓ or j/k] Navigate | [x] Kill Process | [r] Refresh", Style::default().fg(MUTED_GREY)),
    ]);

    let controls_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DARK_BORDER))
        .title(Span::styled(" Process Controls & Sorting ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

    f.render_widget(Paragraph::new(controls_line).block(controls_block), chunks[0]);

    // 2. Main Process Table
    let header_cells = [
        "PID",
        "Type",
        "Name",
        "VRAM Used",
        "% VRAM",
        "Command Line",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)));

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = procs
        .iter()
        .map(|p| {
            let pct = (p.used_memory_bytes as f64 / total_vram as f64) * 100.0;
            let type_color = match p.proc_type {
                crate::telemetry::ProcessType::Compute => ACCENT_CYAN,
                crate::telemetry::ProcessType::Graphics => PURPLE_ACCENT,
                crate::telemetry::ProcessType::ComputeAndGraphics => ALERT_AMBER,
            };

            let cells = vec![
                Cell::from(p.pid.to_string()).style(Style::default().fg(Color::White)),
                Cell::from(p.proc_type.label()).style(Style::default().fg(type_color)),
                Cell::from(p.name.clone()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Cell::from(format_bytes(p.used_memory_bytes)).style(Style::default().fg(ALERT_AMBER)),
                Cell::from(format!("{:.1}%", pct)).style(Style::default().fg(util_color(pct))),
                Cell::from(p.cmdline.clone()).style(Style::default().fg(MUTED_GREY)),
            ];
            Row::new(cells)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(22),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Min(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DARK_BORDER))
            .title(Span::styled(" Active GPU Tasks ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(30, 41, 59))
            .fg(NVIDIA_GREEN)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    f.render_stateful_widget(table, chunks[1], &mut app.process_table_state);

    // 3. Process Inspector Card
    let selected_idx = app.process_table_state.selected().unwrap_or(0);
    let inspector_content = if let Some(selected_proc) = procs.get(selected_idx) {
        vec![
            Line::from(vec![
                Span::styled("Selected: ", Style::default().fg(MUTED_GREY)),
                Span::styled(format!("{} (PID: {})", selected_proc.name, selected_proc.pid), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" | Memory: {}", format_bytes(selected_proc.used_memory_bytes)), Style::default().fg(ALERT_AMBER)),
                Span::styled(" | Action: Press [x] to terminate", Style::default().fg(ALERT_RED)),
            ]),
            Line::from(vec![
                Span::styled("Command:  ", Style::default().fg(MUTED_GREY)),
                Span::styled(selected_proc.cmdline.clone(), Style::default().fg(ACCENT_CYAN)),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled("No process selected", Style::default().fg(MUTED_GREY)))]
    };

    let inspector_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DARK_BORDER))
        .title(Span::styled(" Process Details & Inspector ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

    f.render_widget(Paragraph::new(inspector_content).block(inspector_block), chunks[2]);
}
