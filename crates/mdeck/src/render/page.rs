//! The page: a theme's `page:` block lays the slide on a sheet with a
//! surface around it (paper on a desk, a blueprint on a drafting table).
//! The core draws it for every engine, and everything else (the engine's
//! layer, the slide, the chrome) draws on the sheet.

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::theme::Theme;

/// The sheet inside `rect`: `rect` itself when the theme has no page.
pub fn sheet(rect: Rect, theme: &Theme, scale: f32) -> Rect {
    match &theme.page {
        Some(page) => rect.shrink(page.margin * scale),
        None => rect,
    }
}

/// Paint the surface, the sheet's shadow, the sheet and its grain, and
/// return the sheet. Without a page this paints nothing and returns `rect`.
pub fn draw(painter: &egui::Painter, rect: Rect, theme: &Theme, scale: f32) -> Rect {
    let Some(page) = &theme.page else {
        return rect;
    };
    let sheet = sheet(rect, theme, scale);
    painter.rect_filled(rect, 0.0, page.surface);
    let radius = page.radius * scale;
    if page.shadow > 0.0 {
        let shadow = egui::epaint::Shadow {
            offset: [0, (10.0 * scale * page.shadow) as i8],
            blur: (36.0 * scale * page.shadow).min(255.0) as u8,
            spread: 0,
            color: Color32::from_black_alpha((150.0 * page.shadow).min(255.0) as u8),
        };
        painter.add(shadow.as_shape(sheet, radius));
    }
    painter.rect_filled(sheet, radius, theme.background);
    if page.grain > 0.0 {
        grain(painter, sheet, theme, scale, page.grain);
    }
    sheet
}

/// A fine, fixed speckle of paper fibre, darker and lighter than the sheet.
fn grain(painter: &egui::Painter, sheet: Rect, theme: &Theme, scale: f32, amount: f32) {
    let dark = Color32::from_black_alpha((26.0 * amount) as u8);
    let light = Color32::from_white_alpha((20.0 * amount) as u8);
    let _ = theme;
    let mut mesh = egui::Mesh::default();
    let count = (sheet.width() * sheet.height() / (1400.0 * scale * scale)).min(4000.0) as u32;
    for k in 0..count {
        let x = sheet.left() + crate::engines::hash01(k * 3 + 11) * sheet.width();
        let y = sheet.top() + crate::engines::hash01(k * 3 + 12) * sheet.height();
        let s = (0.8 + 1.6 * crate::engines::hash01(k * 3 + 13)) * scale;
        let c = if k % 3 == 0 { light } else { dark };
        mesh.add_colored_rect(
            Rect::from_min_size(Pos2::new(x, y), egui::vec2(s, s * 0.6)),
            c,
        );
    }
    painter.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_page_means_the_whole_slide() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let theme = Theme::dark();
        assert_eq!(sheet(rect, &theme, 1.0), rect);
        let mut paper = Theme::light();
        paper.page = Some(crate::theme::Page {
            surface: Color32::GRAY,
            margin: 40.0,
            shadow: 0.5,
            grain: 0.5,
            radius: 4.0,
        });
        let s = sheet(rect, &paper, 0.5);
        assert_eq!(s.left(), 20.0);
        assert_eq!(s.width(), 1880.0);
    }
}
