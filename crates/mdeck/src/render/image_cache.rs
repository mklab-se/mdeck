use eframe::egui;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Images whose longest side exceeds this are downscaled before upload. A
/// slide is never shown above 4K, so larger textures only cost VRAM and
/// upload time.
pub const MAX_TEXTURE_SIDE: u32 = 4096;

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

/// A decoded RGBA image produced on a background thread.
struct DecodedImage {
    size: [usize; 2],
    rgba: Vec<u8>,
}

/// Where an image is in its loading lifecycle.
pub enum ImageState {
    /// Decoding on a background thread; a repaint is requested when it lands.
    Loading,
    /// Uploaded to the GPU and ready to draw.
    Ready(egui::TextureHandle),
    /// The file could not be read or decoded.
    Missing,
}

/// Lazily loads images off the UI thread and caches the resulting textures.
///
/// Decoding a large photo can take tens of milliseconds, which would visibly
/// hitch a slide transition if done on the UI thread. Instead the first request
/// spawns a decode thread and reports [`ImageState::Loading`]; the thread asks
/// egui to repaint when the pixels are ready, and the next frame uploads them.
pub struct ImageCache {
    base_path: PathBuf,
    textures: RefCell<HashMap<String, Option<egui::TextureHandle>>>,
    pending: RefCell<HashMap<String, Receiver<Option<DecodedImage>>>>,
}

impl ImageCache {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            textures: RefCell::new(HashMap::new()),
            pending: RefCell::new(HashMap::new()),
        }
    }

    /// Clear all cached textures so images reload on next access.
    pub fn clear(&mut self) {
        self.textures.get_mut().clear();
        self.pending.get_mut().clear();
    }

    /// True while any image is still being decoded.
    pub fn is_loading(&self) -> bool {
        !self.pending.borrow().is_empty()
    }

    /// Get a texture by image path, loading lazily on first access.
    /// Returns `None` while the image is loading or if it failed to load.
    pub fn get_or_load(&self, ui: &egui::Ui, path: &str) -> Option<egui::TextureHandle> {
        match self.state(ui.ctx(), path) {
            ImageState::Ready(texture) => Some(texture),
            ImageState::Loading | ImageState::Missing => None,
        }
    }

    /// Start decoding an image in the background without waiting for it.
    /// Used to warm the cache for upcoming slides.
    pub fn preload(&self, ctx: &egui::Context, path: &str) {
        if self.textures.borrow().contains_key(path) || self.pending.borrow().contains_key(path) {
            return;
        }
        self.spawn_load(ctx, path);
    }

    /// Current state of an image, uploading finished decodes to the GPU.
    pub fn state(&self, ctx: &egui::Context, path: &str) -> ImageState {
        if let Some(entry) = self.textures.borrow().get(path) {
            return match entry {
                Some(texture) => ImageState::Ready(texture.clone()),
                None => ImageState::Missing,
            };
        }

        // Poll the decode thread; keep the borrow short so spawning below
        // can take its own mutable borrow.
        let poll = self.pending.borrow().get(path).map(|rx| rx.try_recv());
        let received = match poll {
            Some(Ok(decoded)) => decoded,
            Some(Err(TryRecvError::Empty)) => return ImageState::Loading,
            Some(Err(TryRecvError::Disconnected)) => None,
            None => {
                self.spawn_load(ctx, path);
                return ImageState::Loading;
            }
        };

        self.pending.borrow_mut().remove(path);
        let texture = received.map(|decoded| {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(decoded.size, &decoded.rgba);
            ctx.load_texture(path, color_image, texture_options())
        });
        self.textures
            .borrow_mut()
            .insert(path.to_string(), texture.clone());
        match texture {
            Some(texture) => ImageState::Ready(texture),
            None => ImageState::Missing,
        }
    }

    fn spawn_load(&self, ctx: &egui::Context, path: &str) {
        let full_path = self.resolve(path);
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let decoded = decode_image(&full_path);
            // The receiver may be gone if the cache was cleared meanwhile.
            let _ = tx.send(decoded);
            ctx.request_repaint();
        });
        self.pending.borrow_mut().insert(path.to_string(), rx);
    }

    /// Resolve relative paths against the presentation's directory.
    fn resolve(&self, path: &str) -> PathBuf {
        if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.base_path.join(path)
        }
    }
}

fn decode_image(path: &Path) -> Option<DecodedImage> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let rgba = downscale_if_needed(img).to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(DecodedImage {
        size,
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_png(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let img = image::RgbaImage::from_pixel(4, 3, image::Rgba([10, 20, 30, 255]));
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn decode_image_reads_png_dimensions() {
        let dir = std::env::temp_dir().join(format!("mdeck-imgcache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_png(&dir, "tiny.png");
        let decoded = decode_image(&path).expect("png decodes");
        assert_eq!(decoded.size, [4, 3]);
        assert_eq!(decoded.rgba.len(), 4 * 3 * 4);
        assert_eq!(&decoded.rgba[..4], &[10, 20, 30, 255]);
        let _ = std::fs::remove_dir_all(&dir);
    }

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

    #[test]
    fn decode_image_missing_file_is_none() {
        assert!(decode_image(Path::new("/definitely/not/here.png")).is_none());
    }

    #[test]
    fn state_transitions_loading_to_ready_or_missing() {
        let dir = std::env::temp_dir().join(format!("mdeck-imgcache2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_png(&dir, "ok.png");
        let cache = ImageCache::new(dir.clone());
        let ctx = egui::Context::default();

        // First call kicks off a background decode.
        assert!(matches!(cache.state(&ctx, "ok.png"), ImageState::Loading));
        assert!(cache.is_loading());

        // Poll until the decode thread delivers.
        let mut ready = false;
        for _ in 0..200 {
            if matches!(cache.state(&ctx, "ok.png"), ImageState::Ready(_)) {
                ready = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(ready, "image should become ready");
        assert!(!cache.is_loading());
        assert!(cache.get_or_load_ctx(&ctx, "ok.png").is_some());

        // A missing file settles into Missing and is remembered.
        let mut missing = false;
        for _ in 0..200 {
            if matches!(cache.state(&ctx, "nope.png"), ImageState::Missing) {
                missing = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(missing);
        assert!(matches!(cache.state(&ctx, "nope.png"), ImageState::Missing));
        let _ = std::fs::remove_dir_all(&dir);
    }

    impl ImageCache {
        fn get_or_load_ctx(&self, ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
            match self.state(ctx, path) {
                ImageState::Ready(t) => Some(t),
                _ => None,
            }
        }
    }
}
