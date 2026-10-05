use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use super::theme::*;
use crate::app::App;
use crate::utils::{format_bytes, format_mhz, format_temp, format_watts};

fn make_mini_bar(pct: f64, width: usize) -> String {
    let clamped = pct.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

pub fn render_all_gpus(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Fleet summary cards
            Constraint::Min(8),    // Multi-GPU hardware table
            Constraint::Length(9), // Fleet-wide processes table
        ])
        .split(area);

    render_fleet_summary(f, app, rows[0]);
    render_gpu_table(f, app, rows[1]);
    render_fleet_processes(f, app, rows[2]);
}

fn render_fleet_summary(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let count = app.device_count;
    let mut total_util = 0.0;
    let mut total_vram_used = 0u64;
    let mut total_vram_cap = 0u64;
    let mut total_power_watts = 0.0;
    let mut total_power_limit = 0.0;
    let mut max_temp = 0u32;
    let mut total_procs = 0usize;
    let mut sampled_count = 0u32;

    for i in 0..count {
        if let Some(snap) = app.all_snapshots.get(&i) {
            total_util += snap.util_gpu;
            total_vram_used += snap.mem_used_bytes;
            total_vram_cap += snap.mem_total_bytes;
            total_power_watts += snap.power_watts;
            total_power_limit += snap.power_limit_watts;
            if snap.temp_c > max_temp {
                max_temp = snap.temp_c;
            }
            total_procs += snap.processes.len();
            sampled_count += 1;
        }
    }

    let avg_util = if sampled_count > 0 {
        total_util / sampled_count as f64
    } else {
        0.0
    };

    let vram_pct = if total_vram_cap > 0 {
        (total_vram_used as f64 / total_vram_cap as f64) * 100.0
    } else {
        0.0
    };

    let power_pct = if total_power_limit > 0.0 {
        (total_power_watts / total_power_limit) * 100.0
    } else {
        0.0
    };

    // Card 1: Fleet Devices & Average Load
    let card1_color = util_color(avg_util);
    let card1_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(card1_color))
        .title(Span::styled(" Fleet Devices & Load ", Style::default().fg(card1_color).add_modifier(Modifier::BOLD)));
    let card1_text = vec![
        Line::from(vec![
            Span::styled(format!("{} Devices Online", count), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" | Avg Load: {:.1}%", avg_util), Style::default().fg(card1_color)),
        ]),
        Line::from(vec![
            Span::styled(format!("[{}] ", make_mini_bar(avg_util, 12)), Style::default().fg(card1_color)),
            Span::styled("Cluster Compute", Style::default().fg(MUTED_GREY)),
        ]),
    ];
    f.render_widget(Paragraph::new(card1_text).block(card1_block), cols[0]);

    // Card 2: Fleet VRAM Allocation
    let card2_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_CYAN))
        .title(Span::styled(" Total VRAM Allocation ", Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD)));
    let card2_text = vec![
        Line::from(vec![
            Span::styled(format!("{} / {}", format_bytes(total_vram_used), format_bytes(total_vram_cap)), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({:.1}%)", vram_pct), Style::default().fg(ACCENT_CYAN)),
        ]),
        Line::from(vec![
            Span::styled(format!("[{}] ", make_mini_bar(vram_pct, 12)), Style::default().fg(ACCENT_CYAN)),
            Span::styled("Fleet Memory", Style::default().fg(MUTED_GREY)),
        ]),
    ];
    f.render_widget(Paragraph::new(card2_text).block(card2_block), cols[1]);

    // Card 3: Fleet Power Envelope
    let card3_color = power_color(total_power_watts, total_power_limit);
    let card3_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(card3_color))
        .title(Span::styled(" Total Power Draw ", Style::default().fg(card3_color).add_modifier(Modifier::BOLD)));
    let card3_text = vec![
        Line::from(vec![
            Span::styled(format!("{} / {}", format_watts(total_power_watts), format_watts(total_power_limit)), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({:.1}%)", power_pct), Style::default().fg(card3_color)),
        ]),
        Line::from(vec![
            Span::styled(format!("[{}] ", make_mini_bar(power_pct, 12)), Style::default().fg(card3_color)),
            Span::styled("Power Envelope", Style::default().fg(MUTED_GREY)),
        ]),
    ];
    f.render_widget(Paragraph::new(card3_text).block(card3_block), cols[2]);

    // Card 4: Thermals & Process Count
    let card4_color = temp_color(max_temp);
    let card4_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(card4_color))
        .title(Span::styled(" Thermals & Processes ", Style::default().fg(card4_color).add_modifier(Modifier::BOLD)));
    let card4_text = vec![
        Line::from(vec![
            Span::styled(format!("Peak Temp: {}", format_temp(max_temp)), Style::default().fg(card4_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" | {} Tasks", total_procs), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(format!("Monitored: {}/{} GPUs", sampled_count, count), Style::default().fg(MUTED_GREY)),
        ]),
    ];
    f.render_widget(Paragraph::new(card4_text).block(card4_block), cols[3]);
}

fn render_gpu_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = [
        "SEL",
        "GPU",
        "DEVICE NAME",
        "COMPUTE UTIL",
        "VRAM ALLOCATION",
        "POWER",
        "TEMP / FAN",
        "CLOCKS",
        "PCIE",
        "PROCS",
    ]
    .into_iter()
    .map(|h| {
        Cell::from(Span::styled(
            h,
            Style::default()
                .fg(ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ))
    });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let mut rows = Vec::new();

    for i in 0..app.device_count {
        let is_selected = i == app.selected_gpu;

        let (sel_marker, sel_style) = if is_selected {
            (
                "►",
                Style::default()
                    .fg(NVIDIA_GREEN)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (" ", Style::default().fg(MUTED_GREY))
        };

        if let Some(snap) = app.all_snapshots.get(&i) {
            let util = snap.util_gpu;
            let u_color = util_color(util);
            let util_bar = format!("[{}] {:>4.1}%", make_mini_bar(util, 8), util);

            let mem_pct = snap.mem_used_percent();
            let vram_bar = format!(
                "[{}] {:>4.1}% ({})",
                make_mini_bar(mem_pct, 8),
                mem_pct,
                format_bytes(snap.mem_used_bytes)
            );

            let p_color = power_color(snap.power_watts, snap.power_limit_watts);
            let power_str = format!(
                "{} / {}",
                format_watts(snap.power_watts),
                format_watts(snap.power_limit_watts)
            );

            let t_color = temp_color(snap.temp_c);
            let fan_str = snap
                .fan_speed_pct
                .map(|f| format!("{}%", f))
                .unwrap_or_else(|| "N/A".to_string());
            let temp_str = format!("{} | Fan {}", format_temp(snap.temp_c), fan_str);

            let clk_str = snap
                .clock_graphics_mhz
                .map(format_mhz)
                .unwrap_or_else(|| "N/A".to_string());

            let pci_gen = snap.pci_link_gen.map(|g| format!("Gen{}", g)).unwrap_or_default();
            let pci_w = snap.pci_link_width.map(|w| format!("x{}", w)).unwrap_or_default();
            let pci_str = if pci_gen.is_empty() && pci_w.is_empty() {
                "N/A".to_string()
            } else {
                format!("{} {}", pci_gen, pci_w)
            };

            let procs_count = format!("{} tasks", snap.processes.len());

            let row_cells = vec![
                Cell::from(Span::styled(sel_marker, sel_style)),
                Cell::from(Span::styled(
                    format!("GPU {}", i),
                    if is_selected {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                )),
                Cell::from(Span::styled(
                    snap.name.clone(),
                    if is_selected {
                        Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                )),
                Cell::from(Span::styled(util_bar, Style::default().fg(u_color))),
                Cell::from(Span::styled(vram_bar, Style::default().fg(ACCENT_CYAN))),
                Cell::from(Span::styled(power_str, Style::default().fg(p_color))),
                Cell::from(Span::styled(temp_str, Style::default().fg(t_color))),
                Cell::from(Span::styled(clk_str, Style::default().fg(MUTED_GREY))),
                Cell::from(Span::styled(pci_str, Style::default().fg(MUTED_GREY))),
                Cell::from(Span::styled(procs_count, Style::default().fg(Color::White))),
            ];

            let row_style = if is_selected {
                Style::default().bg(Color::Rgb(20, 35, 25))
            } else {
                Style::default()
            };

            rows.push(Row::new(row_cells).style(row_style).height(1));
        } else {
            let row_cells = vec![
                Cell::from(Span::styled(sel_marker, sel_style)),
                Cell::from(Span::styled(format!("GPU {}", i), Style::default().fg(Color::White))),
                Cell::from(Span::styled("Polling telemetry...", Style::default().fg(MUTED_GREY))),
                Cell::from(Span::raw("-")),
                Cell::from(Span::raw("-")),
                Cell::from(Span::raw("-")),
                Cell::from(Span::raw("-")),
                Cell::from(Span::raw("-")),
                Cell::from(Span::raw("-")),
                Cell::from(Span::raw("-")),
            ];
            rows.push(Row::new(row_cells).height(1));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DARK_BORDER))
        .title(Span::styled(
            format!(" Monitored GPU Devices ({}) [↑/↓ Select, Enter Overview] ", app.device_count),
            Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD),
        ));

    let widths = [
        Constraint::Length(3),  // SEL
        Constraint::Length(7),  // GPU #
        Constraint::Min(24),    // NAME
        Constraint::Length(19), // COMPUTE UTIL
        Constraint::Length(28), // VRAM ALLOCATION
        Constraint::Length(17), // POWER
        Constraint::Length(16), // TEMP / FAN
        Constraint::Length(11), // CLOCKS
        Constraint::Length(11), // PCIE
        Constraint::Length(10), // PROCS
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block);

    f.render_widget(table, area);
}

fn render_fleet_processes(f: &mut Frame, app: &App, area: Rect) {
    let mut all_procs: Vec<(u32, &crate::telemetry::GpuProcess, u64)> = Vec::new();

    for i in 0..app.device_count {
        if let Some(snap) = app.all_snapshots.get(&i) {
            for p in &snap.processes {
                all_procs.push((i, p, snap.mem_total_bytes));
            }
        }
    }

    // Sort descending by memory usage
    all_procs.sort_by(|a, b| b.1.used_memory_bytes.cmp(&a.1.used_memory_bytes));

    let header_cells = ["GPU", "PID", "TYPE", "VRAM", "% GPU", "PROCESS NAME", "COMMAND LINE"]
        .into_iter()
        .map(|h| {
            Cell::from(Span::styled(
                h,
                Style::default()
                    .fg(ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ))
        });
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = if all_procs.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled("No active compute/graphics processes detected across all GPUs.", Style::default().fg(MUTED_GREY)))
        ])]
    } else {
        all_procs
            .iter()
            .take(6)
            .map(|(gpu_idx, p, total_mem)| {
                let pct = if *total_mem > 0 {
                    (p.used_memory_bytes as f64 / *total_mem as f64) * 100.0
                } else {
                    0.0
                };

                let type_style = match p.proc_type {
                    crate::telemetry::ProcessType::Compute => Style::default().fg(NVIDIA_GREEN),
                    crate::telemetry::ProcessType::Graphics => Style::default().fg(ACCENT_CYAN),
                    crate::telemetry::ProcessType::ComputeAndGraphics => Style::default().fg(PURPLE_ACCENT),
                };

                let cells = vec![
                    Cell::from(Span::styled(format!("GPU {}", gpu_idx), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(format!("{}", p.pid), Style::default().fg(Color::White))),
                    Cell::from(Span::styled(p.proc_type.label(), type_style)),
                    Cell::from(Span::styled(format_bytes(p.used_memory_bytes), Style::default().fg(ACCENT_CYAN))),
                    Cell::from(Span::styled(format!("{:>5.1}%", pct), Style::default().fg(Color::White))),
                    Cell::from(Span::styled(p.name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(
                        if p.cmdline.is_empty() { &p.name } else { &p.cmdline },
                        Style::default().fg(MUTED_GREY),
                    )),
                ];
                Row::new(cells).height(1)
            })
            .collect()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DARK_BORDER))
        .title(Span::styled(
            format!(" Active Processes Across All GPUs ({}) ", all_procs.len()),
            Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD),
        ));

    let widths = [
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(9),
        Constraint::Length(22),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block);

    f.render_widget(table, area);
}
