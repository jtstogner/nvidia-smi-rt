use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, Gauge, GraphType, Paragraph, Row, Sparkline, Table},
    Frame,
};

use super::theme::*;
use crate::app::App;
use crate::utils::{format_bytes, format_mhz, format_temp, format_watts};

pub fn render_overview(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Gauges & quick stats row
            Constraint::Min(10),   // Real-time Braille charts row
            Constraint::Length(9), // Hardware details & top processes row
        ])
        .split(area);

    render_gauges_row(f, app, rows[0]);
    render_charts_row(f, app, rows[1]);
    render_bottom_row(f, app, rows[2]);
}

fn render_gauges_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let Some(snap) = &app.current_snapshot else {
        return;
    };
    let history = app.histories.get(&app.selected_gpu);

    // 1. GPU Core Util Gauge
    {
        let util_pct = snap.util_gpu.clamp(0.0, 100.0);
        let color = util_color(util_pct);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(2)])
            .split(cols[0]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .title(Span::styled(" GPU Compute Util ", Style::default().fg(color).add_modifier(Modifier::BOLD)));

        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(color).bg(DARK_BORDER))
            .percent(util_pct as u16)
            .label(format!("{:.1}%", util_pct));
        f.render_widget(gauge, chunks[0]);

        if let Some(hist) = history {
            let spark_data = hist.gpu_util.to_sparkline_data(100.0);
            let spark = Sparkline::default()
                .data(&spark_data)
                .style(Style::default().fg(color));
            f.render_widget(spark, chunks[1]);
        }
    }

    // 2. VRAM Memory Usage Gauge
    {
        let mem_pct = snap.mem_used_percent().clamp(0.0, 100.0);
        let color = ACCENT_CYAN;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(2)])
            .split(cols[1]);

        let used_str = format_bytes(snap.mem_used_bytes);
        let total_str = format_bytes(snap.mem_total_bytes);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .title(Span::styled(
                format!(" VRAM ({}/{}) ", used_str, total_str),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));

        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(color).bg(DARK_BORDER))
            .percent(mem_pct as u16)
            .label(format!("{:.1}% ({})", mem_pct, used_str));
        f.render_widget(gauge, chunks[0]);

        if let Some(hist) = history {
            let spark_data = hist.mem_util.to_sparkline_data(100.0);
            let spark = Sparkline::default()
                .data(&spark_data)
                .style(Style::default().fg(color));
            f.render_widget(spark, chunks[1]);
        }
    }

    // 3. Power Draw Gauge
    {
        let power_pct = snap.power_percent().clamp(0.0, 100.0);
        let color = power_color(snap.power_watts, snap.power_limit_watts);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(2)])
            .split(cols[2]);

        let watts_str = format_watts(snap.power_watts);
        let limit_str = format_watts(snap.power_limit_watts);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .title(Span::styled(
                format!(" Power Draw ({}/{}) ", watts_str, limit_str),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));

        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(color).bg(DARK_BORDER))
            .percent(power_pct as u16)
            .label(format!("{:.1}% ({})", power_pct, watts_str));
        f.render_widget(gauge, chunks[0]);

        if let Some(hist) = history {
            let spark_data = hist.power_watts.to_sparkline_data(snap.power_limit_watts.max(100.0));
            let spark = Sparkline::default()
                .data(&spark_data)
                .style(Style::default().fg(color));
            f.render_widget(spark, chunks[1]);
        }
    }

    // 4. Thermals & Fan Gauge
    {
        let color = temp_color(snap.temp_c);
        let fan_str = snap
            .fan_speed_pct
            .map(|f| format!("{}%", f))
            .unwrap_or_else(|| "N/A".to_string());

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(2)])
            .split(cols[3]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .title(Span::styled(
                format!(" Thermals & Fan (Fan: {}) ", fan_str),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));

        let temp_pct = ((snap.temp_c as f64 / 100.0) * 100.0).clamp(0.0, 100.0);

        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(color).bg(DARK_BORDER))
            .percent(temp_pct as u16)
            .label(format_temp(snap.temp_c));
        f.render_widget(gauge, chunks[0]);

        if let Some(hist) = history {
            let spark_data = hist.temp_c.to_sparkline_data(100.0);
            let spark = Sparkline::default()
                .data(&spark_data)
                .style(Style::default().fg(color));
            f.render_widget(spark, chunks[1]);
        }
    }
}

fn render_charts_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let Some(hist) = app.histories.get(&app.selected_gpu) else {
        return;
    };

    // Left Chart: GPU Util % (Green) & VRAM % (Cyan)
    {
        let gpu_points: Vec<(f64, f64)> = hist.gpu_util.points.iter().copied().collect();
        let mem_points: Vec<(f64, f64)> = hist.mem_util.points.iter().copied().collect();

        let (_min_g, max_g, avg_g, cur_g) = hist.gpu_util.stats();
        let (_min_m, _max_m, _avg_m, cur_m) = hist.mem_util.stats();

        let x_min = gpu_points.first().map(|p| p.0).unwrap_or(0.0);
        let x_max = gpu_points.last().map(|p| p.0).unwrap_or(100.0).max(x_min + 30.0);

        let datasets = vec![
            Dataset::default()
                .name(format!("GPU: {:.0}% (avg: {:.0}% max: {:.0}%)", cur_g, avg_g, max_g))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(NVIDIA_GREEN))
                .data(&gpu_points),
            Dataset::default()
                .name(format!("VRAM: {:.1}%", cur_m))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(ACCENT_CYAN))
                .data(&mem_points),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_BORDER))
                    .title(Line::from(vec![
                        Span::styled(" Realtime Compute & Memory Utilization (%) ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ])),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::styled("history", Style::default().fg(MUTED_GREY)),
                        Span::styled("now", Style::default().fg(MUTED_GREY)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([0.0, 100.0])
                    .labels(vec![
                        Span::styled("0%", Style::default().fg(MUTED_GREY)),
                        Span::styled("50%", Style::default().fg(MUTED_GREY)),
                        Span::styled("100%", Style::default().fg(MUTED_GREY)),
                    ]),
            );

        f.render_widget(chart, cols[0]);
    }

    // Right Chart: Power Draw (Watts) & Thermals (°C)
    {
        let power_points: Vec<(f64, f64)> = hist.power_watts.points.iter().copied().collect();
        let temp_points: Vec<(f64, f64)> = hist.temp_c.points.iter().copied().collect();

        let (_min_p, max_p, avg_p, cur_p) = hist.power_watts.stats();
        let (_min_t, max_t, avg_t, cur_t) = hist.temp_c.stats();

        let x_min = power_points.first().map(|p| p.0).unwrap_or(0.0);
        let x_max = power_points.last().map(|p| p.0).unwrap_or(100.0).max(x_min + 30.0);

        let max_y = (max_p.max(100.0) * 1.15).ceil();

        let datasets = vec![
            Dataset::default()
                .name(format!("Power: {:.1}W (avg: {:.1}W max: {:.1}W)", cur_p, avg_p, max_p))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(ALERT_AMBER))
                .data(&power_points),
            Dataset::default()
                .name(format!("Temp: {:.0}°C (avg: {:.0}°C max: {:.0}°C)", cur_t, avg_t, max_t))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(ALERT_RED))
                .data(&temp_points),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_BORDER))
                    .title(Line::from(vec![
                        Span::styled(" Realtime Power (W) & Thermal (°C) Dynamics ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ])),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::styled("history", Style::default().fg(MUTED_GREY)),
                        Span::styled("now", Style::default().fg(MUTED_GREY)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([0.0, max_y])
                    .labels(vec![
                        Span::styled("0", Style::default().fg(MUTED_GREY)),
                        Span::styled(format!("{:.0}", max_y / 2.0), Style::default().fg(MUTED_GREY)),
                        Span::styled(format!("{:.0}", max_y), Style::default().fg(MUTED_GREY)),
                    ]),
            );

        f.render_widget(chart, cols[1]);
    }
}

fn render_bottom_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let Some(snap) = &app.current_snapshot else {
        return;
    };

    // Hardware Details Panel
    {
        let gfx_clock = snap
            .clock_graphics_mhz
            .map(format_mhz)
            .unwrap_or_else(|| "N/A".to_string());
        let gfx_max = snap
            .clock_graphics_max_mhz
            .map(format_mhz)
            .unwrap_or_else(|| "N/A".to_string());
        let mem_clock = snap
            .clock_memory_mhz
            .map(format_mhz)
            .unwrap_or_else(|| "N/A".to_string());

        let enc_str = snap
            .util_encoder
            .map(|e| format!("{:.0}%", e))
            .unwrap_or_else(|| "0%".to_string());
        let dec_str = snap
            .util_decoder
            .map(|d| format!("{:.0}%", d))
            .unwrap_or_else(|| "0%".to_string());

        let pci_tx = snap
            .pci_tx_kbytes_per_sec
            .map(|k| format!("{:.2} MB/s", k as f64 / 1024.0))
            .unwrap_or_else(|| "N/A".to_string());
        let pci_rx = snap
            .pci_rx_kbytes_per_sec
            .map(|k| format!("{:.2} MB/s", k as f64 / 1024.0))
            .unwrap_or_else(|| "N/A".to_string());

        let lines = vec![
            Line::from(vec![
                Span::styled("Clocks:       ", Style::default().fg(MUTED_GREY)),
                Span::styled(format!("Graphics: {} (Max: {}) | Memory: {}", gfx_clock, gfx_max, mem_clock), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Video Engine: ", Style::default().fg(MUTED_GREY)),
                Span::styled(format!("Encoder: {} | Decoder: {}", enc_str, dec_str), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("PCIe Traffic: ", Style::default().fg(MUTED_GREY)),
                Span::styled(format!("TX (Host->GPU): {} | RX (GPU->Host): {}", pci_tx, pci_rx), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("State:        ", Style::default().fg(MUTED_GREY)),
                Span::styled(format!("Perf: {} | Persistence: {} | Backend: {}", snap.perf_state, if snap.persistence_mode { "On" } else { "Off" }, app.engine.backend_name()), Style::default().fg(Color::White)),
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DARK_BORDER))
            .title(Span::styled(" Hardware & Engine Telemetry ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

        f.render_widget(Paragraph::new(lines).block(block), cols[0]);
    }

    // Top Processes Table
    {
        let header_cells = ["PID", "Type", "Process Name", "VRAM Usage"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows: Vec<Row> = snap
            .processes
            .iter()
            .take(4)
            .map(|p| {
                let pid_str = p.pid.to_string();
                let type_str = p.proc_type.label();
                let vram_str = format_bytes(p.used_memory_bytes);
                let cells = vec![
                    Cell::from(pid_str).style(Style::default().fg(ACCENT_CYAN)),
                    Cell::from(type_str).style(Style::default().fg(MUTED_GREY)),
                    Cell::from(p.name.clone()).style(Style::default().fg(Color::White)),
                    Cell::from(vram_str).style(Style::default().fg(ALERT_AMBER)),
                ];
                Row::new(cells)
            })
            .collect();

        let proc_count = snap.processes.len();
        let title = format!(" Active GPU Processes ({}) [Press 3 for Process Manager] ", proc_count);

        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(9),
                Constraint::Min(15),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DARK_BORDER))
                .title(Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
        );

        f.render_widget(table, cols[1]);
    }
}
