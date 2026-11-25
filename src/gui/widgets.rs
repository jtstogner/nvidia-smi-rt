use std::collections::VecDeque;

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use super::theme::{with_alpha, BORDER_SUBTLE, SURFACE_CARD, SURFACE_ELEVATED, TEXT_MUTED, TEXT_PRIMARY};

pub fn card_frame(accent_border: Option<Color32>) -> egui::Frame {
    let border = accent_border.unwrap_or(BORDER_SUBTLE);
    egui::Frame::new()
        .fill(SURFACE_CARD)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(12))
}

pub fn pill_badge(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, with_alpha(fg, 120)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(fg).size(11.5).strong());
        });
}

pub fn progress_bar(
    ui: &mut egui::Ui,
    fraction: f32,
    fill: Color32,
    height: f32,
    overlay_text: Option<&str>,
) {
    let clamped = fraction.clamp(0.0, 1.0);
    let desired_width = ui.available_width().max(48.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(desired_width, height), Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same((height * 0.35).round().clamp(3.0, 8.0) as u8);
        painter.rect_filled(rect, radius, SURFACE_ELEVATED);
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, BORDER_SUBTLE),
            egui::StrokeKind::Inside,
        );

        if clamped > 0.005 {
            let fill_w = (rect.width() * clamped).max(height.min(rect.width()));
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
            painter.rect_filled(fill_rect, radius, fill);
        }

        if let Some(label) = overlay_text {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional((height * 0.68).clamp(10.0, 13.0)),
                TEXT_PRIMARY,
            );
        }
    }
}

pub fn sparkline(
    ui: &mut egui::Ui,
    points: &VecDeque<(f64, f64)>,
    max_y: f64,
    color: Color32,
    height: f32,
) {
    let desired_width = ui.available_width().max(60.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(desired_width, height), Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), with_alpha(SURFACE_ELEVATED, 140));

    let len = points.len();
    if len == 0 {
        return;
    }

    let pad_y = 3.0;
    let inner = rect.shrink2(Vec2::new(2.0, pad_y));
    let scale_y = max_y.max(1e-6);

    let to_pos = |idx: usize, val: f64| -> Pos2 {
        let x_t = if len <= 1 {
            1.0
        } else {
            idx as f32 / (len - 1) as f32
        };
        let y_t = (val / scale_y).clamp(0.0, 1.0) as f32;
        Pos2::new(
            inner.left() + x_t * inner.width(),
            inner.bottom() - y_t * inner.height(),
        )
    };

    let line_pts: Vec<Pos2> = points
        .iter()
        .enumerate()
        .map(|(i, &(_, y))| to_pos(i, y))
        .collect();

    if line_pts.len() >= 2 {
        let mut area_pts = line_pts.clone();
        area_pts.push(Pos2::new(line_pts.last().unwrap().x, inner.bottom()));
        area_pts.push(Pos2::new(line_pts.first().unwrap().x, inner.bottom()));
        painter.add(egui::Shape::convex_polygon(
            area_pts,
            with_alpha(color, 35),
            Stroke::NONE,
        ));
        painter.add(egui::Shape::line(line_pts.clone(), Stroke::new(1.6, color)));
    }

    if let Some(&last) = line_pts.last() {
        painter.circle_filled(last, 2.5, color);
    }
}

pub fn kpi_metric_card(
    ui: &mut egui::Ui,
    title: &str,
    right_subtitle: &str,
    primary_value: &str,
    secondary_value: &str,
    fraction: f32,
    accent: Color32,
    spark_points: Option<&VecDeque<(f64, f64)>>,
    spark_max_y: f64,
) {
    card_frame(Some(with_alpha(accent, 110))).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).color(accent).strong().size(12.5));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(right_subtitle).color(TEXT_MUTED).size(11.5));
            });
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(primary_value).color(TEXT_PRIMARY).strong().size(22.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(secondary_value).color(TEXT_MUTED).size(12.0));
            });
        });

        ui.add_space(6.0);
        progress_bar(ui, fraction, accent, 8.0, None);

        ui.add_space(6.0);
        if let Some(pts) = spark_points {
            sparkline(ui, pts, spark_max_y, accent, 28.0);
        } else {
            let empty = VecDeque::new();
            sparkline(ui, &empty, spark_max_y, accent, 28.0);
        }
    });
}
