//! Notes pages for `mdeck export --format pdf --notes`: the slide on top and
//! its speaker notes below, on a portrait page in A-series proportions (it
//! prints on A4 and fits Letter). Notes pages are always dark text on white,
//! whatever the deck's theme, so they print well; the slide keeps its theme.
//! Notes that do not fit continue on further pages rather than shrinking.

use std::ops::Range;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2, pos2, vec2};

use crate::parser::{self, Block};
use crate::render::image_cache::ImageCache;
use crate::render::text;
use crate::theme::Theme;

/// Page height over width, as for A4.
pub const PAGE_RATIO: f32 = std::f32::consts::SQRT_2;

/// Where things go on a notes page, in canvas pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    pub width: u32,
    pub height: u32,
    /// The slide image on the first page.
    pub slide: Rect,
    /// Left edge and width of the notes text.
    pub text_x: f32,
    pub text_width: f32,
    /// Where notes start on the first page and on continuation pages.
    pub text_top_first: f32,
    pub text_top_rest: f32,
    /// Notes end above this line; the footer sits below it.
    pub text_bottom: f32,
    /// Scale for the light theme's text sizes (body text is about 10 pt
    /// when the page is printed on A4).
    pub scale: f32,
}

impl Geometry {
    pub fn new(width: u32, slide_w: u32, slide_h: u32, theme: &Theme) -> Self {
        let w = width as f32;
        let height = (w * PAGE_RATIO).round() as u32;
        let h = height as f32;
        let margin = (w * 0.07).round();
        let slide_w_px = w - 2.0 * margin;
        let slide_h_px = (slide_w_px * slide_h as f32 / slide_w.max(1) as f32).round();
        let slide = Rect::from_min_size(pos2(margin, margin), vec2(slide_w_px, slide_h_px));
        let footer = (w * 0.05).round();
        Self {
            width,
            height,
            slide,
            text_x: margin,
            text_width: slide_w_px,
            text_top_first: slide.bottom() + (w * 0.045).round(),
            text_top_rest: margin,
            text_bottom: h - margin * 0.6 - footer,
            scale: (w / 58.0) / theme.body_size,
        }
    }

    fn footer_y(&self) -> f32 {
        self.height as f32 - self.text_x * 0.6 - self.width as f32 * 0.02
    }
}

/// The printable part of a slide's notes: text-like blocks only (charts,
/// diagrams and images in notes are left out).
pub fn blocks(notes: Option<&str>) -> Vec<Block> {
    let Some(notes) = notes else {
        return Vec::new();
    };
    parser::blocks::parse(notes)
        .into_iter()
        .filter(|b| {
            matches!(
                b,
                Block::Heading { .. }
                    | Block::Paragraph { .. }
                    | Block::List { .. }
                    | Block::CodeBlock { .. }
                    | Block::BlockQuote { .. }
                    | Block::Table { .. }
                    | Block::HorizontalRule
            )
        })
        .collect()
}

/// Split blocks with the given heights (each followed by its gap) into pages
/// holding at most `first` pixels on the first page and `rest` on the others.
/// Every page takes at least one block, so a block taller than a page gets a
/// page of its own. There is always at least one page.
pub fn paginate(heights: &[(f32, f32)], first: f32, rest: f32) -> Vec<Range<usize>> {
    let mut pages = Vec::new();
    let mut start = 0;
    let mut used = 0.0;
    let mut capacity = first;
    for (i, &(h, gap)) in heights.iter().enumerate() {
        if i > start && used + h > capacity {
            pages.push(start..i);
            start = i;
            used = 0.0;
            capacity = rest;
        }
        used += h + gap;
    }
    pages.push(start..heights.len());
    pages
}

/// Measure every block's height and following gap at the page's scale.
pub fn measure(ui: &egui::Ui, blocks: &[Block], geo: &Geometry, theme: &Theme) -> Vec<(f32, f32)> {
    blocks
        .iter()
        .map(|b| {
            (
                text::measure_single_block_height(ui, b, theme, geo.text_width, geo.scale),
                text::block_spacing(b, theme, geo.scale),
            )
        })
        .collect()
}

/// Draw one notes page, shifted by `offset` (the tile being captured).
#[allow(clippy::too_many_arguments)]
pub fn draw(
    ui: &egui::Ui,
    offset: Vec2,
    geo: &Geometry,
    theme: &Theme,
    blocks: &[Block],
    range: Range<usize>,
    first_page: bool,
    footer_left: &str,
    footer_right: &str,
) {
    let painter = ui.painter();
    let page = Rect::from_min_size(Pos2::ZERO, vec2(geo.width as f32, geo.height as f32));
    painter.rect_filled(page.translate(offset), 0.0, Color32::WHITE);
    let hairline = Color32::from_gray(0xC8);
    if first_page {
        // The slide image is composited here after capture; the frame shows
        // light slides against the white page.
        painter.rect_stroke(
            geo.slide.expand(1.0).translate(offset),
            0.0,
            Stroke::new(1.5, hairline),
            egui::StrokeKind::Outside,
        );
    }
    let top = if first_page {
        geo.text_top_first
    } else {
        geo.text_top_rest
    };
    let images = ImageCache::new(std::path::PathBuf::new());
    text::draw_blocks(
        ui,
        &blocks[range],
        theme,
        pos2(geo.text_x, top) + offset,
        geo.text_width,
        1.0,
        &images,
        usize::MAX,
        geo.scale,
    );

    let size = theme.body_size * geo.scale * 0.62;
    let font = egui::FontId::new(size, theme.body_family());
    let grey = Color32::from_gray(0x8A);
    let y = geo.footer_y();
    painter.hline(
        geo.text_x..=geo.text_x + geo.text_width,
        y - size * 0.9 + offset.y,
        Stroke::new(1.0, Color32::from_gray(0xE0)),
    );
    painter.text(
        pos2(geo.text_x, y) + offset,
        egui::Align2::LEFT_TOP,
        footer_left,
        font.clone(),
        grey,
    );
    painter.text(
        pos2(geo.text_x + geo.text_width, y) + offset,
        egui::Align2::RIGHT_TOP,
        footer_right,
        font,
        grey,
    );
}

/// Scale the rendered slide into `rect` on the page canvas (RGBA, `page_w`
/// pixels wide).
pub fn composite(
    page: &mut [u8],
    page_w: u32,
    slide: &[u8],
    slide_w: u32,
    slide_h: u32,
    rect: Rect,
) {
    let (w, h) = (rect.width().round() as u32, rect.height().round() as u32);
    let Some(src) = image::RgbaImage::from_raw(slide_w, slide_h, slide.to_vec()) else {
        return;
    };
    let scaled = image::imageops::resize(
        &src,
        w.max(1),
        h.max(1),
        image::imageops::FilterType::Lanczos3,
    );
    let (ox, oy) = (rect.min.x.round() as u32, rect.min.y.round() as u32);
    let page_h = page.len() as u32 / 4 / page_w.max(1);
    for y in 0..scaled.height().min(page_h.saturating_sub(oy)) {
        for x in 0..scaled.width().min(page_w.saturating_sub(ox)) {
            let d = (((oy + y) * page_w + ox + x) * 4) as usize;
            page[d..d + 4].copy_from_slice(&scaled.get_pixel(x, y).0);
        }
    }
}

/// The current slide's notes, laid out for printing.
pub struct NotesJob {
    pub geo: Geometry,
    pub blocks: Vec<Block>,
    /// Block ranges per page; empty until measured on the first notes frame.
    pub pages: Vec<Range<usize>>,
    /// The rendered slide, composited onto the first notes page.
    pub slide: Vec<u8>,
}

impl NotesJob {
    /// Draw page `page` of these notes for the tile at `origin`, measuring
    /// and paginating on the first call.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        origin: (u32, u32),
        page: usize,
        theme: &Theme,
        deck_title: &str,
        slide: usize,
        count: usize,
    ) {
        if self.pages.is_empty() {
            let heights = measure(ui, &self.blocks, &self.geo, theme);
            self.pages = paginate(
                &heights,
                self.geo.text_bottom - self.geo.text_top_first,
                self.geo.text_bottom - self.geo.text_top_rest,
            );
        }
        let range = self.pages.get(page).cloned().unwrap_or(0..0);
        let right = if page == 0 {
            format!("Slide {slide} of {count}")
        } else {
            format!("Slide {slide} of {count}, notes continued")
        };
        draw(
            ui,
            egui::vec2(-(origin.0 as f32), -(origin.1 as f32)),
            &self.geo,
            theme,
            &self.blocks,
            range,
            page == 0,
            deck_title,
            &right,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_is_portrait_a_series_with_the_slide_on_top() {
        let theme = Theme::light();
        let g = Geometry::new(1920, 1920, 1080, &theme);
        assert_eq!(g.height, 2715);
        assert!((g.slide.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);
        assert!(g.text_top_first > g.slide.bottom());
        assert!(g.text_bottom > g.text_top_first + g.height as f32 * 0.3);
        // body text about 1/58 of the page width
        assert!((theme.body_size * g.scale - 1920.0 / 58.0).abs() < 0.01);
    }

    #[test]
    fn paginate_packs_greedily_and_never_loses_a_block() {
        // no notes: one empty page
        assert_eq!(paginate(&[], 100.0, 200.0), vec![0..0]);
        // everything fits on the first page
        assert_eq!(
            paginate(&[(30.0, 10.0), (30.0, 10.0)], 100.0, 200.0),
            vec![0..2]
        );
        // the third block spills over; later pages are larger
        assert_eq!(
            paginate(
                &[
                    (40.0, 10.0),
                    (40.0, 10.0),
                    (40.0, 10.0),
                    (90.0, 10.0),
                    (90.0, 10.0)
                ],
                100.0,
                200.0
            ),
            vec![0..2, 2..4, 4..5]
        );
        // a block taller than any page gets its own page
        assert_eq!(
            paginate(&[(10.0, 5.0), (500.0, 5.0), (10.0, 5.0)], 100.0, 200.0),
            vec![0..1, 1..2, 2..3]
        );
    }

    #[test]
    fn notes_keep_text_blocks_and_drop_visuals() {
        let b = blocks(Some(
            "Say **this**.\n\n- one\n- two\n\n```@barchart\n- A: 1\n```\n\n![x](y.png)\n",
        ));
        assert_eq!(b.len(), 2, "{b:?}");
        assert!(blocks(None).is_empty());
    }

    #[test]
    fn composite_scales_the_slide_into_its_rect() {
        let (pw, ph) = (10u32, 10u32);
        let mut page = vec![255u8; (pw * ph * 4) as usize];
        let slide = [0u8, 0, 255, 255].repeat(4 * 2);
        composite(
            &mut page,
            pw,
            &slide,
            4,
            2,
            Rect::from_min_size(pos2(2.0, 3.0), vec2(6.0, 3.0)),
        );
        let px = |x: u32, y: u32| {
            let i = ((y * pw + x) * 4) as usize;
            [page[i], page[i + 1], page[i + 2]]
        };
        assert_eq!(px(1, 1), [255, 255, 255]);
        assert_eq!(px(4, 4), [0, 0, 255]);
        assert_eq!(px(7, 5), [0, 0, 255]);
        assert_eq!(px(8, 6), [255, 255, 255]);
    }
}
