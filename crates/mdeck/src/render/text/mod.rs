//! Markdown blocks drawn as text: inline runs, headings, paragraphs, lists,
//! code, tables and images, one block at a time or stacked in a flow.
//! Measurement lays everything out exactly as drawing does, so overflow
//! detection agrees with what is on screen.

mod block;
mod code;
mod image;
mod inline;
mod list;
mod mono;
mod quote;
mod table;

pub use block::{block_spacing, draw_block, draw_blocks, measure_single_block_height};
pub(crate) use code::{CODE_PADDING, widest_code_line};
pub use image::{draw_image_in_area, image_rect_in};
pub use mono::{append_code, settle as settle_code};
