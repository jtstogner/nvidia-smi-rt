use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame,
};

use super::theme::*;
use crate::app::App;

pub fn render_charts_tab(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let Some(hist) = app.histories.get(&app.selected_gpu) else {
        return;
    };
    let snap = app.current_snapshot.as_ref();

    // 1. Chart Top Left: GPU Core Compute Utilization (%)
    {
        let pts: Vec<(f64, f64)> = hist.gpu_util.points.iter().copied().collect();
        let (min, max, avg, cur) = hist.gpu_util.stats();

        let x_min = pts.first().map(|p| p.0).unwrap_or(0.0);
        let x_max = pts.last().map(|p| p.0).unwrap_or(100.0).max(x_min + 30.0);

        let datasets = vec![
            Dataset::default()
                .name(format!("Current: {:.1}% | Avg: {:.1}% | Min: {:.1}% | Max: {:.1}%", cur, avg, min, max))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(NVIDIA_GREEN))
                .data(&pts),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_BORDER))
                    .title(Line::from(vec![
                        Span::styled(" [1] GPU Core Compute Utilization (%) ", Style::default().fg(NVIDIA_GREEN).add_modifier(Modifier::BOLD)),
                    ])),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::styled("t-history", Style::default().fg(MUTED_GREY)),
                        Span::styled("now", Style::default().fg(MUTED_GREY)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([0.0, 100.0])
                    .labels(vec![
                        Span::styled("0%", Style::default().fg(MUTED_GREY)),
                        Span::styled("25%", Style::default().fg(MUTED_GREY)),
                        Span::styled("50%", Style::default().fg(MUTED_GREY)),
                        Span::styled("75%", Style::default().fg(MUTED_GREY)),
                        Span::styled("100%", Style::default().fg(MUTED_GREY)),
                    ]),
            );

        f.render_widget(chart, top_cols[0]);
    }

    // 2. Chart Top Right: VRAM Allocation (GiB)
    {
        let pts: Vec<(f64, f64)> = hist.mem_used_gib.points.iter().copied().collect();
        let (_min, max, avg, cur) = hist.mem_used_gib.stats();

        let total_gib = snap
            .map(|s| s.mem_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            .unwrap_or(24.0);

        let x_min = pts.first().map(|p| p.0).unwrap_or(0.0);
        let x_max = pts.last().map(|p| p.0).unwrap_or(100.0).max(x_min + 30.0);

        let datasets = vec![
            Dataset::default()
                .name(format!("VRAM Used: {:.2} GiB (Avg: {:.2} | Max: {:.2} / Total: {:.1} GiB)", cur, avg, max, total_gib))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(ACCENT_CYAN))
                .data(&pts),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_BORDER))
                    .title(Line::from(vec![
                        Span::styled(" [2] VRAM Allocation & Memory Footprint (GiB) ", Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD)),
                    ])),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::styled("t-history", Style::default().fg(MUTED_GREY)),
                        Span::styled("now", Style::default().fg(MUTED_GREY)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([0.0, total_gib])
                    .labels(vec![
                        Span::styled("0 GiB", Style::default().fg(MUTED_GREY)),
                        Span::styled(format!("{:.0} GiB", total_gib / 2.0), Style::default().fg(MUTED_GREY)),
                        Span::styled(format!("{:.0} GiB", total_gib), Style::default().fg(MUTED_GREY)),
                    ]),
            );

        f.render_widget(chart, top_cols[1]);
    }

    // 3. Chart Bottom Left: Power Draw vs Power Cap (Watts)
    {
        let pts: Vec<(f64, f64)> = hist.power_watts.points.iter().copied().collect();
        let (_min, max, avg, cur) = hist.power_watts.stats();

        let cap = snap.map(|s| s.power_limit_watts).unwrap_or(450.0).max(100.0);

        let x_min = pts.first().map(|p| p.0).unwrap_or(0.0);
        let x_max = pts.last().map(|p| p.0).unwrap_or(100.0).max(x_min + 30.0);

        let datasets = vec![
            Dataset::default()
                .name(format!("Power: {:.1}W | Avg: {:.1}W | Max: {:.1}W (Cap: {:.0}W)", cur, avg, max, cap))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(ALERT_AMBER))
                .data(&pts),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_BORDER))
                    .title(Line::from(vec![
                        Span::styled(" [3] Realtime Power Draw (Watts) ", Style::default().fg(ALERT_AMBER).add_modifier(Modifier::BOLD)),
                    ])),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::styled("t-history", Style::default().fg(MUTED_GREY)),
                        Span::styled("now", Style::default().fg(MUTED_GREY)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([0.0, cap])
                    .labels(vec![
                        Span::styled("0W", Style::default().fg(MUTED_GREY)),
                        Span::styled(format!("{:.0}W", cap / 2.0), Style::default().fg(MUTED_GREY)),
                        Span::styled(format!("{:.0}W", cap), Style::default().fg(MUTED_GREY)),
                    ]),
            );

        f.render_widget(chart, bottom_cols[0]);
    }

    // 4. Chart Bottom Right: Core Thermals (°C) & Graphics Clock (MHz)
    {
        let temp_pts: Vec<(f64, f64)> = hist.temp_c.points.iter().copied().collect();
        let (_min_t, max_t, avg_t, cur_t) = hist.temp_c.stats();

        let (_min_c, max_c, _avg_c, cur_c) = hist.clock_graphics.stats();

        let x_min = temp_pts.first().map(|p| p.0).unwrap_or(0.0);
        let x_max = temp_pts.last().map(|p| p.0).unwrap_or(100.0).max(x_min + 30.0);

        let datasets = vec![
            Dataset::default()
                .name(format!("Temp: {:.0}°C (Avg: {:.0}°C Max: {:.0}°C) | Clock: {:.0} MHz (Max: {:.0} MHz)", cur_t, avg_t, max_t, cur_c, max_c))
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(ALERT_RED))
                .data(&temp_pts),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_BORDER))
                    .title(Line::from(vec![
                        Span::styled(" [4] GPU Thermals (°C) & Graphics Engine ", Style::default().fg(ALERT_RED).add_modifier(Modifier::BOLD)),
                    ])),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::styled("t-history", Style::default().fg(MUTED_GREY)),
                        Span::styled("now", Style::default().fg(MUTED_GREY)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(MUTED_GREY))
                    .bounds([0.0, 105.0])
                    .labels(vec![
                        Span::styled("0°C", Style::default().fg(MUTED_GREY)),
                        Span::styled("50°C", Style::default().fg(MUTED_GREY)),
                        Span::styled("75°C", Style::default().fg(MUTED_GREY)),
                        Span::styled("100°C", Style::default().fg(MUTED_GREY)),
                    ]),
            );

        f.render_widget(chart, bottom_cols[1]);
    }
}
