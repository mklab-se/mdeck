//! Golden images: compare a rendered frame with a PNG committed next to the
//! test, and regenerate it on request.

use std::path::{Path, PathBuf};

use crate::paint::ImageData;

/// Set this environment variable to `1` to rewrite every golden image a
/// test run compares against: `MDECK_UPDATE_GOLDEN=1 cargo test`.
pub const UPDATE_GOLDEN_ENV: &str = "MDECK_UPDATE_GOLDEN";

/// The default tolerance for [`assert_golden`]: a mean channel difference
/// of 1.5 (out of 255) absorbs small rasterisation changes but catches a
/// moved, recoloured or missing shape.
pub const GOLDEN_TOLERANCE: f32 = 1.5;

fn updating() -> bool {
    std::env::var(UPDATE_GOLDEN_ENV).is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Write `image` as a PNG (straight alpha, 8 bits per channel), creating
/// the parent folder if needed.
///
/// ```
/// use mdeck_sdk::paint::{Color, ImageData};
/// use mdeck_sdk::testing::{read_png, write_png};
/// let path = std::env::temp_dir().join("mdeck-sdk-doc-png/red.png");
/// let img = ImageData::filled([3, 2], Color::from_rgb(255, 0, 0));
/// write_png(&path, &img).unwrap();
/// assert_eq!(read_png(&path).unwrap(), img);
/// ```
pub fn write_png(path: impl AsRef<Path>, image: &ImageData) -> std::io::Result<()> {
    let path = path.as_ref();
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir)?;
    }
    let file = std::fs::File::create(path)?;
    let [w, h] = image.size;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::High);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|c| c.to_rgba_unmultiplied())
        .collect();
    writer
        .write_image_data(&bytes)
        .map_err(std::io::Error::other)?;
    writer.finish().map_err(std::io::Error::other)
}

/// Read an 8-bit PNG (as [`write_png`] writes them; grey and RGB images
/// are read as opaque).
///
/// See [`write_png`] for an example.
pub fn read_png(path: impl AsRef<Path>) -> std::io::Result<ImageData> {
    let file = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(std::io::Error::other)?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buf).map_err(std::io::Error::other)?;
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..w * h * 4].to_vec(),
        png::ColorType::Rgb => buf[..w * h * 3]
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf[..w * h * 2]
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf[..w * h].iter().flat_map(|&g| [g, g, g, 255]).collect(),
        other => {
            return Err(std::io::Error::other(format!(
                "unsupported PNG colour type {other:?}"
            )));
        }
    };
    ImageData::from_rgba_unmultiplied([w, h], &rgba)
        .ok_or_else(|| std::io::Error::other("PNG size does not match its data"))
}

/// Compare `actual` with the golden PNG at `path` and panic when they differ
/// by more than `tolerance` (a [`super::mean_difference`]; see
/// [`GOLDEN_TOLERANCE`]).
///
/// - With [`UPDATE_GOLDEN_ENV`] set to anything but `0`, the golden image
///   is (re)written and the comparison passes.
/// - When the golden image does not exist yet it is written and the test
///   passes, so a new test records its first image. Under CI (the `CI`
///   environment variable is set) a missing golden image fails instead.
/// - On a mismatch the rendered image is written next to the golden one as
///   `<name>.actual.png`, so you can look at both.
///
/// ```
/// use mdeck_sdk::paint::{Color, ImageData};
/// use mdeck_sdk::testing::{assert_golden, write_png, GOLDEN_TOLERANCE};
/// let path = std::env::temp_dir().join("mdeck-sdk-doc-golden/white.png");
/// let img = ImageData::filled([4, 4], Color::WHITE);
/// write_png(&path, &img).unwrap();
/// assert_golden(&path, &img, GOLDEN_TOLERANCE);
/// ```
#[track_caller]
pub fn assert_golden(path: impl AsRef<Path>, actual: &ImageData, tolerance: f32) {
    let path = path.as_ref();
    let update = updating();
    if update || (!path.exists() && std::env::var_os("CI").is_none()) {
        write_png(path, actual)
            .unwrap_or_else(|e| panic!("cannot write golden image {}: {e}", path.display()));
        if !update {
            eprintln!("recorded new golden image {}", path.display());
        }
        return;
    }
    let expected = read_png(path).unwrap_or_else(|e| {
        panic!(
            "cannot read golden image {}: {e} (run with {UPDATE_GOLDEN_ENV}=1 to record it)",
            path.display()
        )
    });
    let fail = |why: String| -> ! {
        let actual_path = actual_path(path);
        let _ = write_png(&actual_path, actual);
        panic!(
            "{} differs from its golden image: {why}; the rendered frame is in {} \
             (run with {UPDATE_GOLDEN_ENV}=1 to accept it)",
            path.display(),
            actual_path.display()
        );
    };
    match super::mean_difference(&expected, actual) {
        None => fail(format!(
            "size {:?} instead of {:?}",
            actual.size, expected.size
        )),
        Some(d) if d > tolerance => fail(format!(
            "mean difference {d:.2} is over the tolerance {tolerance:.2}"
        )),
        Some(_) => {}
    }
}

fn actual_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{stem}.actual.png"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Color;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("mdeck-sdk-golden-{}", std::process::id()))
            .join(name)
    }

    #[test]
    fn premultiplied_pixels_round_trip_through_straight_alpha() {
        let path = temp("half.png");
        let img = ImageData::from_rgba_unmultiplied([1, 1], &[200, 100, 50, 128]).unwrap();
        write_png(&path, &img).unwrap();
        assert_eq!(read_png(&path).unwrap(), img);
    }

    #[test]
    fn a_different_image_fails_and_leaves_the_actual_next_to_it() {
        if updating() {
            return; // updating skips the comparison by design
        }
        let path = temp("diff.png");
        write_png(&path, &ImageData::filled([4, 4], Color::BLACK)).unwrap();
        let white = ImageData::filled([4, 4], Color::WHITE);
        assert!(std::panic::catch_unwind(|| assert_golden(&path, &white, 1.0)).is_err());
        let actual = actual_path(&path);
        assert!(actual.exists());
        assert_eq!(actual.file_name().unwrap(), "diff.actual.png");
    }

    #[test]
    fn a_wrong_size_fails_and_a_close_image_passes() {
        if updating() {
            return;
        }
        let path = temp("size.png");
        write_png(&path, &ImageData::filled([4, 4], Color::BLACK)).unwrap();
        let small = ImageData::filled([2, 2], Color::BLACK);
        assert!(std::panic::catch_unwind(|| assert_golden(&path, &small, 1.0)).is_err());
        let close = ImageData::filled([4, 4], Color::from_gray(1));
        assert_golden(&path, &close, 1.0);
    }
}
