use eframe::egui;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Images whose longest side exceeds this are downscaled before upload. A
/// slide is never shown above 4K, so larger textures only cost VRAM and
/// upload time.
pub const MAX_TEXTURE_SIDE: u32 = 4096;

pub struct ImageCache {
    base_path: PathBuf,
    textures: RefCell<HashMap<String, Option<egui::TextureHandle>>>,
}

impl ImageCache {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            textures: RefCell::new(HashMap::new()),
        }
    }

    /// Clear all cached textures so images reload on next access.
    pub fn clear(&mut self) {
        self.textures.get_mut().clear();
    }

    /// Get a texture by image path, loading lazily on first access.
    pub fn get_or_load(&self, ui: &egui::Ui, path: &str) -> Option<egui::TextureHandle> {
        let mut cache = self.textures.borrow_mut();

        if let Some(entry) = cache.get(path) {
            return entry.clone();
        }

        // Resolve relative paths against base_path
        let full_path = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.base_path.join(path)
        };

        let texture = load_texture(ui, &full_path, path);
        cache.insert(path.to_string(), texture.clone());
        texture
    }
}

/// Texture options for photos: trilinear filtering with mipmaps so images
/// stay smooth when drawn far below their native size (grid overview).
fn texture_options() -> egui::TextureOptions {
    egui::TextureOptions {
        magnification: egui::TextureFilter::Linear,
        minification: egui::TextureFilter::Linear,
        wrap_mode: egui::TextureWrapMode::ClampToEdge,
        mipmap_mode: Some(egui::TextureFilter::Linear),
    }
}

/// Shrink an image so its longest side is at most [`MAX_TEXTURE_SIDE`],
/// preserving aspect ratio. Smaller images are returned untouched.
fn downscale_if_needed(img: image::DynamicImage) -> image::DynamicImage {
    if img.width() > MAX_TEXTURE_SIDE || img.height() > MAX_TEXTURE_SIDE {
        img.thumbnail(MAX_TEXTURE_SIDE, MAX_TEXTURE_SIDE)
    } else {
        img
    }
}

fn load_texture(ui: &egui::Ui, path: &Path, name: &str) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let rgba = downscale_if_needed(img).to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let pixels = rgba.into_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
    let texture = ui.ctx().load_texture(name, color_image, texture_options());
    Some(texture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_images_are_downscaled_preserving_aspect() {
        let img = image::DynamicImage::new_rgba8(8192, 2048);
        let out = downscale_if_needed(img);
        assert_eq!(out.width(), MAX_TEXTURE_SIDE);
        assert_eq!(out.height(), 1024);
    }

    #[test]
    fn small_images_are_left_alone() {
        let img = image::DynamicImage::new_rgba8(640, 480);
        let out = downscale_if_needed(img);
        assert_eq!((out.width(), out.height()), (640, 480));
    }

    #[test]
    fn textures_use_mipmaps() {
        let opts = texture_options();
        assert_eq!(opts.mipmap_mode, Some(egui::TextureFilter::Linear));
        assert_eq!(opts.minification, egui::TextureFilter::Linear);
    }
}
