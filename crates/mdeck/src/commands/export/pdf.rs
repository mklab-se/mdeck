//! Assembling a PDF from rendered pages.
//!
//! Every page is one full-bleed image, so the PDF looks exactly like the
//! slides (fonts, charts, math, the Ember field). Pages are compressed as they
//! arrive and only the compressed bytes are kept, so a long deck does not hold
//! every frame in memory. Each slide gets an outline entry (bookmark) named
//! after its heading.

use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, TextStr};

/// PDF points per rendered pixel: a 1920 px wide slide becomes a 960 pt
/// (13.33 in) page, PowerPoint's widescreen size, at 144 dpi.
pub const PT_PER_PX: f32 = 0.5;

struct Page {
    /// zlib-compressed RGB samples.
    data: Vec<u8>,
    width: u32,
    height: u32,
    /// Outline entry that should point at this page, if any.
    bookmark: Option<String>,
}

/// Document metadata written to the PDF's info dictionary.
#[derive(Default)]
pub struct Meta {
    pub title: Option<String>,
    pub author: Option<String>,
}

#[derive(Default)]
pub struct PdfDoc {
    pages: Vec<Page>,
}

impl PdfDoc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Add a page from RGBA pixels (alpha is ignored: renders are opaque).
    /// `bookmark` names an outline entry that jumps to this page.
    pub fn add_page(&mut self, rgba: &[u8], width: u32, height: u32, bookmark: Option<String>) {
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for px in rgba.as_chunks::<4>().0 {
            rgb.extend_from_slice(&px[..3]);
        }
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        // Writing into a Vec cannot fail.
        enc.write_all(&rgb).expect("in-memory compression");
        let data = enc.finish().expect("in-memory compression");
        self.pages.push(Page {
            data,
            width,
            height,
            bookmark,
        });
    }

    /// The finished PDF file.
    pub fn finish(self, meta: &Meta) -> Vec<u8> {
        let mut pdf = Pdf::new();
        let mut next = 1;
        let mut alloc = || {
            let r = Ref::new(next);
            next += 1;
            r
        };
        let catalog_id = alloc();
        let tree_id = alloc();
        let info_id = alloc();
        let outline_id = alloc();

        // (page, content, image) ids per page
        let ids: Vec<(Ref, Ref, Ref)> = self
            .pages
            .iter()
            .map(|_| (alloc(), alloc(), alloc()))
            .collect();
        let bookmarks: Vec<(usize, &str, Ref)> = self
            .pages
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.bookmark.as_deref().map(|b| (i, b)))
            .map(|(i, b)| (i, b, alloc()))
            .collect();

        let mut catalog = pdf.catalog(catalog_id);
        catalog.pages(tree_id);
        if !bookmarks.is_empty() {
            catalog.outlines(outline_id);
        }
        catalog.finish();
        pdf.pages(tree_id)
            .kids(ids.iter().map(|&(p, _, _)| p))
            .count(ids.len() as i32);

        let image_name = Name(b"Im1");
        for (page, &(page_id, content_id, image_id)) in self.pages.iter().zip(&ids) {
            let (w, h) = (
                page.width as f32 * PT_PER_PX,
                page.height as f32 * PT_PER_PX,
            );
            let mut p = pdf.page(page_id);
            p.media_box(Rect::new(0.0, 0.0, w, h));
            p.parent(tree_id);
            p.contents(content_id);
            p.resources().x_objects().pair(image_name, image_id);
            p.finish();

            let mut image = pdf.image_xobject(image_id, &page.data);
            image.filter(Filter::FlateDecode);
            image.width(page.width as i32);
            image.height(page.height as i32);
            image.color_space().device_rgb();
            image.bits_per_component(8);
            // Smooth scaling when a viewer zooms out.
            image.interpolate(true);
            image.finish();

            let mut content = Content::new();
            content.save_state();
            content.transform([w, 0.0, 0.0, h, 0.0, 0.0]);
            content.x_object(image_name);
            content.restore_state();
            pdf.stream(content_id, &content.finish());
        }

        if let (Some(first), Some(last)) = (bookmarks.first(), bookmarks.last()) {
            pdf.outline(outline_id)
                .first(first.2)
                .last(last.2)
                .count(bookmarks.len() as i32);
            for (k, &(page, title, id)) in bookmarks.iter().enumerate() {
                let mut item = pdf.outline_item(id);
                item.title(TextStr(title));
                item.parent(outline_id);
                if k > 0 {
                    item.prev(bookmarks[k - 1].2);
                }
                if let Some(n) = bookmarks.get(k + 1) {
                    item.next(n.2);
                }
                item.dest().page(ids[page].0).fit();
            }
        }

        let mut info = pdf.document_info(info_id);
        if let Some(t) = &meta.title {
            info.title(TextStr(t));
        }
        if let Some(a) = &meta.author {
            info.author(TextStr(a));
        }
        info.creator(TextStr(&format!("mdeck {}", env!("CARGO_PKG_VERSION"))));
        info.finish();

        pdf.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        rgba.repeat((w * h) as usize)
    }

    fn count(haystack: &[u8], needle: &str) -> usize {
        haystack
            .windows(needle.len())
            .filter(|w| *w == needle.as_bytes())
            .count()
    }

    #[test]
    fn pages_bookmarks_and_metadata_are_written() {
        let mut doc = PdfDoc::new();
        doc.add_page(&solid(4, 2, [255, 0, 0, 255]), 4, 2, Some("Intro".into()));
        doc.add_page(&solid(4, 2, [0, 0, 255, 255]), 4, 2, None);
        doc.add_page(
            &solid(2, 3, [0, 255, 0, 255]),
            2,
            3,
            Some("Näst sista 中文".into()),
        );
        assert_eq!(doc.page_count(), 3);
        let bytes = doc.finish(&Meta {
            title: Some("Deck".into()),
            author: Some("Ada".into()),
        });
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.ends_with(b"%%EOF") || bytes.ends_with(b"%%EOF\n"));
        assert_eq!(
            count(&bytes, "/Type /Page\n") + count(&bytes, "/Type /Page "),
            3
        );
        assert_eq!(count(&bytes, "/Count 3"), 1, "page tree");
        assert_eq!(count(&bytes, "/Count 2"), 1, "outline with two bookmarks");
        // page sizes follow the pixel sizes at PT_PER_PX
        assert_eq!(count(&bytes, "/MediaBox [0 0 2 1]"), 2);
        assert_eq!(count(&bytes, "/MediaBox [0 0 1 1.5]"), 1);
        assert!(count(&bytes, "(Deck)") == 1 && count(&bytes, "(Ada)") == 1);
    }

    #[test]
    fn no_bookmarks_means_no_outline() {
        let mut doc = PdfDoc::new();
        doc.add_page(&solid(2, 2, [0, 0, 0, 255]), 2, 2, None);
        let bytes = doc.finish(&Meta::default());
        assert_eq!(count(&bytes, "/Outlines"), 0);
    }

    #[test]
    fn page_samples_round_trip_through_zlib_as_rgb() {
        use std::io::Read;
        let mut doc = PdfDoc::new();
        doc.add_page(&[1, 2, 3, 255, 4, 5, 6, 255], 2, 1, None);
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(&doc.pages[0].data[..])
            .read_to_end(&mut out)
            .unwrap();
        assert_eq!(out, vec![1, 2, 3, 4, 5, 6]);
    }
}
