mod image;
mod text;
mod file;

pub use text::TextExtractor;
pub use image::ImageExtractor;
pub use file::FileExtractor;

use crate::types::ClipboardExtractor;

pub fn get_all() -> Vec<Box<dyn ClipboardExtractor>> {
    vec![
        Box::new(file::FileExtractor::new()),
        Box::new(image::ImageExtractor::new()),
        Box::new(text::TextExtractor::new()),
    ]
}