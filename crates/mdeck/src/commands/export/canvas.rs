//! Stitching window-sized screenshots into a page of any size.

use std::path::Path;

use eframe::egui;

/// Accumulates screenshot tiles into a single RGBA canvas of the requested size.
///
/// The export window can never be larger than the screen, and on HiDPI
/// displays its pixel size differs from its logical size. Instead of trusting
/// the window, the page is rendered at one point per pixel into a virtual
/// canvas of exactly `width`×`height`, one window-sized tile at a time, and
/// the tiles are stitched together here.
pub struct TileCanvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl TileCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// Copy a screenshot captured for the tile whose top-left canvas pixel is
    /// `(ox, oy)`. Pixels outside the canvas are ignored.
    pub fn blit(&mut self, image: &egui::ColorImage, ox: u32, oy: u32) {
        let copy_w = image.width().min(self.width.saturating_sub(ox) as usize);
        let copy_h = image.height().min(self.height.saturating_sub(oy) as usize);
        for y in 0..copy_h {
            let src_row = y * image.width();
            let dst_row = ((oy as usize + y) * self.width as usize + ox as usize) * 4;
            for x in 0..copy_w {
                let c = image.pixels[src_row + x];
                let d = dst_row + x * 4;
                self.pixels[d..d + 4].copy_from_slice(&[c.r(), c.g(), c.b(), c.a()]);
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        image::save_buffer(
            path,
            &self.pixels,
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| format!("Failed to save {}: {e}", path.display()))
    }
}

/// Number of tiles needed to cover `total` pixels with tiles of `tile` pixels.
pub fn tile_count(total: u32, tile: u32) -> u32 {
    if tile == 0 {
        return 1;
    }
    total.div_ceil(tile).max(1)
}
