use std::f32::consts::PI;

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Stroke};

// ─── Geometric icon fallbacks ────────────────────────────────────────────────

/// What every fallback icon draws with.
struct Pen<'a> {
    painter: &'a egui::Painter,
    center: Pos2,
    /// The radius the icon draws within.
    s: f32,
    color: Color32,
    stroke_width: f32,
    stroke: Stroke,
    // Corner radii and small gaps are in design pixels; scaled like everything else
    r2: f32,
    r3: f32,
    r4: f32,
}

impl Pen<'_> {
    fn line(&self, points: Vec<Pos2>) {
        self.painter.add(egui::Shape::line(points, self.stroke));
    }

    fn segment(&self, a: Pos2, b: Pos2) {
        self.painter.line_segment([a, b], self.stroke);
    }

    fn outline(&self, rect: egui::Rect, radius: f32) {
        self.painter
            .rect_stroke(rect, radius, self.stroke, egui::StrokeKind::Outside);
    }

    /// A point offset from the centre by fractions of the icon radius.
    fn at(&self, dx: f32, dy: f32) -> Pos2 {
        Pos2::new(self.center.x + self.s * dx, self.center.y + self.s * dy)
    }
}

type IconFn = fn(&Pen);

/// The built-in icon names (spec 8.8) and how to draw each; anything else
/// draws `box`.
const ICONS: &[(&str, IconFn)] = &[
    ("user", user),
    ("server", server),
    ("database", database),
    ("cloud", cloud),
    ("lock", lock),
    ("api", api),
    ("cache", cache),
    ("queue", envelope),
    ("mail", envelope),
    ("monitor", monitor),
    ("browser", monitor),
    ("mobile", mobile),
    ("storage", nested),
    ("container", nested),
    ("function", function),
    ("network", network),
    ("key", key),
    ("logs", logs),
];

fn icon_drawer(name: &str) -> Option<IconFn> {
    ICONS.iter().find(|(n, _)| *n == name).map(|&(_, f)| f)
}

/// Draw the line-art icon `icon` centred on `center` within `size`.
pub(super) fn draw_icon_fallback(
    painter: &egui::Painter,
    icon: &str,
    center: Pos2,
    size: f32,
    color: Color32,
    scale: f32,
) {
    let stroke_width = 2.0 * scale;
    let pen = Pen {
        painter,
        center,
        s: size * 0.4,
        color,
        stroke_width,
        stroke: Stroke::new(stroke_width, color),
        r2: 2.0 * scale,
        r3: 3.0 * scale,
        r4: 4.0 * scale,
    };
    icon_drawer(icon).unwrap_or(generic_box)(&pen);
}

fn user(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Circle head
    let head_r = s * 0.35;
    let head_center = Pos2::new(c.x, c.y - s * 0.25);
    p.painter.circle_stroke(head_center, head_r, p.stroke);
    // Body arc (shoulders)
    let body_top = c.y + s * 0.15;
    let body_w = s * 0.6;
    let pts: Vec<Pos2> = (0..=8)
        .map(|i| {
            let t = PI * i as f32 / 8.0;
            Pos2::new(c.x - body_w * t.cos(), body_top + body_w * 0.5 * t.sin())
        })
        .collect();
    p.line(pts);
}

fn server(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Stacked rectangles
    let w = s * 0.7;
    let h = s * 0.25;
    for i in 0..3 {
        let y = c.y - s * 0.45 + i as f32 * (h + p.r2);
        let rect =
            egui::Rect::from_center_size(Pos2::new(c.x, y + h / 2.0), egui::vec2(w * 2.0, h));
        p.outline(rect, p.r2);
        // Small indicator dot
        p.painter.circle_filled(
            Pos2::new(rect.right() - h * 0.4, rect.center().y),
            h * 0.15,
            p.color,
        );
    }
}

fn database(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Cylinder: top ellipse + sides + bottom ellipse
    let w = s * 0.6;
    let h = s * 0.7;
    let ey = s * 0.2; // ellipse vertical radius
    let top_y = c.y - h / 2.0;
    let bot_y = c.y + h / 2.0;

    // Side lines
    p.segment(Pos2::new(c.x - w, top_y), Pos2::new(c.x - w, bot_y));
    p.segment(Pos2::new(c.x + w, top_y), Pos2::new(c.x + w, bot_y));

    // Top ellipse (full)
    let top_pts: Vec<Pos2> = (0..=20)
        .map(|i| {
            let t = 2.0 * PI * i as f32 / 20.0;
            Pos2::new(c.x + w * t.cos(), top_y + ey * t.sin())
        })
        .collect();
    p.line(top_pts);

    // Bottom ellipse (half, lower arc only)
    let bot_pts: Vec<Pos2> = (0..=10)
        .map(|i| {
            let t = PI * i as f32 / 10.0;
            Pos2::new(c.x - w * t.cos(), bot_y + ey * t.sin())
        })
        .collect();
    p.line(bot_pts);
}

fn cloud(p: &Pen) {
    // Overlapping circles
    let r = p.s * 0.28;
    let offsets = [
        (-0.35, 0.1),
        (0.35, 0.1),
        (0.0, -0.2),
        (-0.2, 0.0),
        (0.2, 0.0),
    ];
    for (dx, dy) in offsets {
        p.painter.circle_stroke(p.at(dx, dy), r, p.stroke);
    }
}

fn lock(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Padlock: rectangle body + arc shackle
    let body_w = s * 0.6;
    let body_h = s * 0.5;
    let body_top = c.y;
    let body_rect = egui::Rect::from_min_size(
        Pos2::new(c.x - body_w, body_top),
        egui::vec2(body_w * 2.0, body_h),
    );
    p.outline(body_rect, p.r3);

    // Shackle arc
    let shackle_pts: Vec<Pos2> = (0..=10)
        .map(|i| {
            let t = PI * i as f32 / 10.0;
            Pos2::new(
                c.x + body_w * 0.6 * t.cos(),
                body_top - body_w * 0.6 * t.sin(),
            )
        })
        .collect();
    p.line(shackle_pts);
}

fn api(p: &Pen) {
    let c = p.center;
    // Hexagon
    let r = p.s * 0.55;
    let pts: Vec<Pos2> = (0..6)
        .map(|i| {
            let angle = PI / 6.0 + PI * 2.0 * i as f32 / 6.0;
            Pos2::new(c.x + r * angle.cos(), c.y + r * angle.sin())
        })
        .collect();
    p.painter.add(egui::Shape::closed_line(pts, p.stroke));
}

fn cache(p: &Pen) {
    // Lightning bolt
    let pts = vec![
        p.at(0.1, -0.5),
        p.at(-0.2, 0.05),
        p.at(0.05, 0.05),
        p.at(-0.1, 0.5),
    ];
    p.painter.add(egui::Shape::line(
        pts,
        Stroke::new(p.stroke_width * 1.5, p.color),
    ));
}

fn envelope(p: &Pen) {
    let (c, s) = (p.center, p.s);
    let w = s * 0.65;
    let h = s * 0.45;
    let rect = egui::Rect::from_center_size(c, egui::vec2(w * 2.0, h * 2.0));
    p.outline(rect, p.r2);
    // V flap
    p.line(vec![
        rect.left_top(),
        Pos2::new(c.x, c.y + h * 0.3),
        rect.right_top(),
    ]);
}

fn monitor(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Monitor/screen
    let w = s * 0.7;
    let h = s * 0.5;
    let screen =
        egui::Rect::from_center_size(Pos2::new(c.x, c.y - s * 0.1), egui::vec2(w * 2.0, h * 2.0));
    p.outline(screen, p.r3);
    // Stand
    let stand_y = screen.bottom() + p.r2;
    p.segment(Pos2::new(c.x, stand_y), Pos2::new(c.x, stand_y + s * 0.25));
    p.segment(
        Pos2::new(c.x - s * 0.35, stand_y + s * 0.25),
        Pos2::new(c.x + s * 0.35, stand_y + s * 0.25),
    );
}

fn mobile(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Phone outline
    let w = s * 0.35;
    let h = s * 0.7;
    let rect = egui::Rect::from_center_size(c, egui::vec2(w * 2.0, h * 2.0));
    p.outline(rect, p.r4);
    // Home button
    p.painter
        .circle_stroke(Pos2::new(c.x, rect.bottom() - s * 0.15), s * 0.08, p.stroke);
}

fn nested(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Nested rectangles
    let outer = egui::Rect::from_center_size(c, egui::vec2(s * 1.2, s * 1.0));
    let inner = egui::Rect::from_center_size(c, egui::vec2(s * 0.7, s * 0.55));
    p.outline(outer, p.r3);
    p.outline(inner, p.r2);
}

fn function(p: &Pen) {
    let c = p.center;
    // f(x): lambda symbol
    let font = FontId::new(p.s * 1.2, FontFamily::Monospace);
    let galley = p.painter.layout_no_wrap("λ".to_string(), font, p.color);
    let text_pos = Pos2::new(
        c.x - galley.rect.width() / 2.0,
        c.y - galley.rect.height() / 2.0,
    );
    p.painter.galley(text_pos, galley, p.color);
}

fn network(p: &Pen) {
    // Three connected dots
    let positions = [p.at(0.0, -0.4), p.at(-0.4, 0.3), p.at(0.4, 0.3)];
    for &pos in &positions {
        p.painter.circle_filled(pos, p.s * 0.12, p.color);
    }
    for i in 0..3 {
        p.segment(positions[i], positions[(i + 1) % 3]);
    }
}

fn key(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Key shape: circle + stem
    let head_r = s * 0.25;
    let head_center = Pos2::new(c.x - s * 0.2, c.y);
    p.painter.circle_stroke(head_center, head_r, p.stroke);
    let stem_start = Pos2::new(head_center.x + head_r, c.y);
    let stem_end = Pos2::new(c.x + s * 0.5, c.y);
    p.segment(stem_start, stem_end);
    // Teeth
    p.segment(
        Pos2::new(stem_end.x - s * 0.1, c.y),
        Pos2::new(stem_end.x - s * 0.1, c.y + s * 0.15),
    );
    p.segment(stem_end, Pos2::new(stem_end.x, c.y + s * 0.15));
}

fn logs(p: &Pen) {
    let (c, s) = (p.center, p.s);
    // Stacked lines (like a document)
    let w = s * 0.55;
    let rect = egui::Rect::from_center_size(c, egui::vec2(w * 2.0, s * 1.2));
    p.outline(rect, p.r2);
    for i in 0..4 {
        let y = rect.top() + s * 0.2 + i as f32 * s * 0.25;
        let line_w = if i == 2 { w * 1.2 } else { w * 1.6 };
        p.painter.line_segment(
            [
                Pos2::new(c.x - line_w / 2.0, y),
                Pos2::new(c.x + line_w / 2.0, y),
            ],
            Stroke::new(p.stroke_width * 0.7, p.color),
        );
    }
}

fn generic_box(p: &Pen) {
    // Default: simple rounded rectangle
    let rect = egui::Rect::from_center_size(p.center, egui::vec2(p.s * 1.0, p.s * 0.8));
    p.outline(rect, p.r4);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_icon_has_a_drawing() {
        // The names spec section 8.8 promises; `box` is the fallback itself
        let spec = [
            "user",
            "server",
            "database",
            "cloud",
            "browser",
            "mobile",
            "api",
            "queue",
            "cache",
            "storage",
            "function",
            "container",
            "network",
            "lock",
            "key",
            "mail",
            "logs",
            "monitor",
        ];
        for name in spec {
            assert!(icon_drawer(name).is_some(), "no drawing for {name}");
        }
        assert_eq!(ICONS.len(), spec.len());
        assert!(icon_drawer("box").is_none());
        assert!(icon_drawer("unknown").is_none());
    }
}
