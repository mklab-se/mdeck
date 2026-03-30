use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Stroke};

// ─── Geometric icon fallbacks ────────────────────────────────────────────────

pub(super) fn draw_icon_fallback(
    painter: &egui::Painter,
    icon: &str,
    center: Pos2,
    size: f32,
    color: Color32,
    stroke_width: f32,
) {
    let s = size * 0.4; // icon draws within this radius
    let stroke = Stroke::new(stroke_width, color);

    match icon {
        "user" => {
            // Circle head
            let head_r = s * 0.35;
            let head_center = Pos2::new(center.x, center.y - s * 0.25);
            painter.circle_stroke(head_center, head_r, stroke);
            // Body arc (shoulders)
            let body_top = center.y + s * 0.15;
            let body_w = s * 0.6;
            let pts: Vec<Pos2> = (0..=8)
                .map(|i| {
                    let t = std::f32::consts::PI * i as f32 / 8.0;
                    Pos2::new(
                        center.x - body_w * t.cos(),
                        body_top + body_w * 0.5 * t.sin(),
                    )
                })
                .collect();
            painter.add(egui::Shape::line(pts, stroke));
        }
        "server" => {
            // Stacked rectangles
            let w = s * 0.7;
            let h = s * 0.25;
            for i in 0..3 {
                let y = center.y - s * 0.45 + i as f32 * (h + 2.0);
                let rect = egui::Rect::from_center_size(
                    Pos2::new(center.x, y + h / 2.0),
                    egui::vec2(w * 2.0, h),
                );
                painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Outside);
                // Small indicator dot
                painter.circle_filled(
                    Pos2::new(rect.right() - h * 0.4, rect.center().y),
                    h * 0.15,
                    color,
                );
            }
        }
        "database" => {
            // Cylinder: top ellipse + sides + bottom ellipse
            let w = s * 0.6;
            let h = s * 0.7;
            let ey = s * 0.2; // ellipse vertical radius
            let top_y = center.y - h / 2.0;
            let bot_y = center.y + h / 2.0;

            // Side lines
            painter.line_segment(
                [
                    Pos2::new(center.x - w, top_y),
                    Pos2::new(center.x - w, bot_y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + w, top_y),
                    Pos2::new(center.x + w, bot_y),
                ],
                stroke,
            );

            // Top ellipse (full)
            let top_pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let t = 2.0 * std::f32::consts::PI * i as f32 / 20.0;
                    Pos2::new(center.x + w * t.cos(), top_y + ey * t.sin())
                })
                .collect();
            painter.add(egui::Shape::line(top_pts, stroke));

            // Bottom ellipse (half, lower arc only)
            let bot_pts: Vec<Pos2> = (0..=10)
                .map(|i| {
                    let t = std::f32::consts::PI * i as f32 / 10.0;
                    Pos2::new(center.x - w * t.cos(), bot_y + ey * t.sin())
                })
                .collect();
            painter.add(egui::Shape::line(bot_pts, stroke));
        }
        "cloud" => {
            // Overlapping circles
            let r = s * 0.28;
            let offsets = [
                (-0.35, 0.1),
                (0.35, 0.1),
                (0.0, -0.2),
                (-0.2, 0.0),
                (0.2, 0.0),
            ];
            for (dx, dy) in offsets {
                painter.circle_stroke(Pos2::new(center.x + s * dx, center.y + s * dy), r, stroke);
            }
        }
        "lock" => {
            // Padlock: rectangle body + arc shackle
            let body_w = s * 0.6;
            let body_h = s * 0.5;
            let body_top = center.y;
            let body_rect = egui::Rect::from_min_size(
                Pos2::new(center.x - body_w, body_top),
                egui::vec2(body_w * 2.0, body_h),
            );
            painter.rect_stroke(body_rect, 3.0, stroke, egui::StrokeKind::Outside);

            // Shackle arc
            let shackle_pts: Vec<Pos2> = (0..=10)
                .map(|i| {
                    let t = std::f32::consts::PI * i as f32 / 10.0;
                    Pos2::new(
                        center.x + body_w * 0.6 * t.cos(),
                        body_top - body_w * 0.6 * t.sin(),
                    )
                })
                .collect();
            painter.add(egui::Shape::line(shackle_pts, stroke));
        }
        "api" => {
            // Hexagon
            let r = s * 0.55;
            let pts: Vec<Pos2> = (0..6)
                .map(|i| {
                    let angle =
                        std::f32::consts::PI / 6.0 + std::f32::consts::PI * 2.0 * i as f32 / 6.0;
                    Pos2::new(center.x + r * angle.cos(), center.y + r * angle.sin())
                })
                .collect();
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        "cache" => {
            // Lightning bolt
            let pts = vec![
                Pos2::new(center.x + s * 0.1, center.y - s * 0.5),
                Pos2::new(center.x - s * 0.2, center.y + s * 0.05),
                Pos2::new(center.x + s * 0.05, center.y + s * 0.05),
                Pos2::new(center.x - s * 0.1, center.y + s * 0.5),
            ];
            painter.add(egui::Shape::line(
                pts,
                Stroke::new(stroke_width * 1.5, color),
            ));
        }
        "queue" | "mail" => {
            // Envelope shape
            let w = s * 0.65;
            let h = s * 0.45;
            let rect = egui::Rect::from_center_size(center, egui::vec2(w * 2.0, h * 2.0));
            painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Outside);
            // V flap
            painter.add(egui::Shape::line(
                vec![
                    rect.left_top(),
                    Pos2::new(center.x, center.y + h * 0.3),
                    rect.right_top(),
                ],
                stroke,
            ));
        }
        "monitor" | "browser" => {
            // Monitor/screen
            let w = s * 0.7;
            let h = s * 0.5;
            let screen = egui::Rect::from_center_size(
                Pos2::new(center.x, center.y - s * 0.1),
                egui::vec2(w * 2.0, h * 2.0),
            );
            painter.rect_stroke(screen, 3.0, stroke, egui::StrokeKind::Outside);
            // Stand
            let stand_y = screen.bottom() + 2.0;
            painter.line_segment(
                [
                    Pos2::new(center.x, stand_y),
                    Pos2::new(center.x, stand_y + s * 0.25),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.35, stand_y + s * 0.25),
                    Pos2::new(center.x + s * 0.35, stand_y + s * 0.25),
                ],
                stroke,
            );
        }
        "mobile" => {
            // Phone outline
            let w = s * 0.35;
            let h = s * 0.7;
            let rect = egui::Rect::from_center_size(center, egui::vec2(w * 2.0, h * 2.0));
            painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Outside);
            // Home button
            painter.circle_stroke(
                Pos2::new(center.x, rect.bottom() - s * 0.15),
                s * 0.08,
                stroke,
            );
        }
        "storage" | "container" => {
            // Nested rectangles
            let outer = egui::Rect::from_center_size(center, egui::vec2(s * 1.2, s * 1.0));
            let inner = egui::Rect::from_center_size(center, egui::vec2(s * 0.7, s * 0.55));
            painter.rect_stroke(outer, 3.0, stroke, egui::StrokeKind::Outside);
            painter.rect_stroke(inner, 2.0, stroke, egui::StrokeKind::Outside);
        }
        "function" => {
            // f(x) — lambda symbol
            let font = FontId::new(s * 1.2, FontFamily::Monospace);
            let galley = painter.layout_no_wrap("λ".to_string(), font, color);
            let text_pos = Pos2::new(
                center.x - galley.rect.width() / 2.0,
                center.y - galley.rect.height() / 2.0,
            );
            painter.galley(text_pos, galley, color);
        }
        "network" => {
            // Three connected dots
            let positions = [
                Pos2::new(center.x, center.y - s * 0.4),
                Pos2::new(center.x - s * 0.4, center.y + s * 0.3),
                Pos2::new(center.x + s * 0.4, center.y + s * 0.3),
            ];
            for &p in &positions {
                painter.circle_filled(p, s * 0.12, color);
            }
            for i in 0..3 {
                painter.line_segment([positions[i], positions[(i + 1) % 3]], stroke);
            }
        }
        "key" => {
            // Key shape: circle + stem
            let head_r = s * 0.25;
            let head_center = Pos2::new(center.x - s * 0.2, center.y);
            painter.circle_stroke(head_center, head_r, stroke);
            let stem_start = Pos2::new(head_center.x + head_r, center.y);
            let stem_end = Pos2::new(center.x + s * 0.5, center.y);
            painter.line_segment([stem_start, stem_end], stroke);
            // Teeth
            painter.line_segment(
                [
                    Pos2::new(stem_end.x - s * 0.1, center.y),
                    Pos2::new(stem_end.x - s * 0.1, center.y + s * 0.15),
                ],
                stroke,
            );
            painter.line_segment(
                [stem_end, Pos2::new(stem_end.x, center.y + s * 0.15)],
                stroke,
            );
        }
        "logs" => {
            // Stacked lines (like a document)
            let w = s * 0.55;
            let rect = egui::Rect::from_center_size(center, egui::vec2(w * 2.0, s * 1.2));
            painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Outside);
            for i in 0..4 {
                let y = rect.top() + s * 0.2 + i as f32 * s * 0.25;
                let line_w = if i == 2 { w * 1.2 } else { w * 1.6 };
                painter.line_segment(
                    [
                        Pos2::new(center.x - line_w / 2.0, y),
                        Pos2::new(center.x + line_w / 2.0, y),
                    ],
                    Stroke::new(stroke_width * 0.7, color),
                );
            }
        }
        _ => {
            // Default: simple rounded rectangle
            let rect = egui::Rect::from_center_size(center, egui::vec2(s * 1.0, s * 0.8));
            painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Outside);
        }
    }
}
