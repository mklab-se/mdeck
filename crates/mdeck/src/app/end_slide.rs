//! The virtual slide after the last: "The End", or the engine's end act.

use eframe::egui;

use super::*;

impl PresentationApp {
    pub(super) fn draw_end_slide(&mut self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        if self.theme.engine.capabilities().end_act {
            self.draw_end_caption(ui, rect, scale);
            return;
        }
        // Draw ESC hint at top like regular slides
        let hint_color = egui::Color32::from_gray(100);
        let hint_galley = ui.painter().layout_no_wrap(
            "Press ESC to exit".to_string(),
            egui::FontId::proportional(14.0 * scale),
            hint_color,
        );
        ui.painter().galley(
            egui::pos2(
                rect.center().x - hint_galley.rect.width() / 2.0,
                rect.top() + 20.0 * scale,
            ),
            hint_galley,
            hint_color,
        );

        // "The End" centered — large enough to read from distance
        let title_color = egui::Color32::from_gray(220);
        let galley = ui.painter().layout_no_wrap(
            "The End".to_string(),
            egui::FontId::proportional(140.0 * scale),
            title_color,
        );
        let title_pos = egui::pos2(
            rect.center().x - galley.rect.width() / 2.0,
            rect.center().y - galley.rect.height() / 2.0 - 40.0 * scale,
        );
        ui.painter().galley(title_pos, galley, title_color);

        // Bottom-right attribution block: logo + text
        let margin = 32.0 * scale;
        let logo_height = 48.0 * scale;

        // Load logo texture lazily
        if self.end_logo_texture.is_none() {
            static LOGO_BYTES: &[u8] = include_bytes!("../../media/logo-small.png");
            if let Ok(img) = image::load_from_memory(LOGO_BYTES) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
                let texture = ui.ctx().load_texture(
                    "mdeck-end-logo",
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.end_logo_texture = Some(texture);
            }
        }

        let text_color = egui::Color32::from_gray(140);
        let url_color = egui::Color32::from_gray(100);

        let powered_galley = ui.painter().layout_no_wrap(
            "Powered by MDeck".to_string(),
            egui::FontId::proportional(14.0 * scale),
            text_color,
        );
        let url_galley = ui.painter().layout_no_wrap(
            "https://github.com/mklab-se/mdeck".to_string(),
            egui::FontId::proportional(11.0 * scale),
            url_color,
        );

        // Position: bottom-right corner
        let text_block_width = powered_galley.rect.width().max(url_galley.rect.width());
        let logo_aspect = 192.0 / 128.0; // width/height of the embedded logo
        let logo_width = logo_height * logo_aspect;

        let block_width = logo_width + 10.0 * scale + text_block_width;
        let block_x = rect.right() - margin - block_width;
        let block_y = rect.bottom() - margin - logo_height;

        // Draw logo
        if let Some(ref texture) = self.end_logo_texture {
            let logo_rect = egui::Rect::from_min_size(
                egui::pos2(block_x, block_y),
                egui::vec2(logo_width, logo_height),
            );
            // Rounded clip for the logo
            ui.painter()
                .rect_filled(logo_rect, 6.0 * scale, egui::Color32::from_gray(30));
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter()
                .image(texture.id(), logo_rect, uv, egui::Color32::WHITE);
        }

        // Draw text lines to the right of logo
        let text_x = block_x + logo_width + 10.0 * scale;
        let text_y = block_y
            + (logo_height - powered_galley.rect.height() - url_galley.rect.height() - 4.0 * scale)
                / 2.0;

        ui.painter()
            .galley(egui::pos2(text_x, text_y), powered_galley, text_color);
        ui.painter().galley(
            egui::pos2(text_x, text_y + 14.0 * scale + 4.0 * scale),
            url_galley,
            url_color,
        );
    }

    /// An engine's end slide: the engine plays its own end act (see
    /// `engines::Host`), so only a quiet caption is drawn, once the act is over.
    fn draw_end_caption(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        let delay = self.theme.engine.end_caption_delay();
        let alpha = ((self.deck.engine.end_elapsed() - delay) / 0.9).clamp(0.0, 1.0);
        ui.ctx().request_repaint();
        if alpha <= 0.0 {
            return;
        }
        let painter = ui.painter();
        let size = 15.0 * scale;
        let font = egui::FontId::new(size, self.theme.mono_family());
        let mut job = egui::text::LayoutJob::default();
        job.append(
            "POWERED BY MDECK  ·  GITHUB.COM/MKLAB-SE/MDECK",
            0.0,
            egui::text::TextFormat {
                font_id: font,
                color: Theme::with_opacity(egui::Color32::from_rgb(0x8F, 0x8F, 0x98), alpha),
                extra_letter_spacing: size * 0.22,
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        painter.galley(
            egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.center().y - galley.rect.height() / 2.0,
            ),
            galley,
            egui::Color32::WHITE,
        );
    }
}
