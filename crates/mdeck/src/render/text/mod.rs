//! Markdown blocks drawn as text: inline runs, headings, paragraphs, lists,
//! code, tables and images, one block at a time or stacked in a flow.
//! Measurement lays everything out exactly as drawing does, so overflow
//! detection agrees with what is on screen.

mod block;
mod code;
mod image;
mod inline;
mod list;
mod table;

pub use block::{
    block_spacing, draw_block, draw_blocks, heading_spacing, measure_blocks_height,
    measure_single_block_height,
};
pub(crate) use code::{CODE_PADDING, measure_code_block_height, widest_code_line};
pub use image::draw_image_in_area;
pub use inline::{display_inlines_job, draw_heading, inlines_to_job};
pub(crate) use list::item_step;
