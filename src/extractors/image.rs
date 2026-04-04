use std::borrow::Cow;
use std::io::Cursor;
use std::sync::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use arboard::{Clipboard, ImageData};
use image::{ImageBuffer, ImageFormat, Rgba};
use crate::types::{ClipboardExtractor, ContentType};
use crate::database::{ClipboardDb, HistoryEntry};

pub struct ImageExtractor {
    last_hash: Mutex<u64>,
}

impl ImageExtractor {
    pub fn new() -> Self {
        Self { last_hash: Mutex::new(0) }
    }

    fn calculate_hash(bytes: &[u8]) -> u64 {
        let mut s = DefaultHasher::new();
        bytes.hash(&mut s);
        s.finish()
    }
}

impl ClipboardExtractor for ImageExtractor {
    fn try_save(&self, cb: &mut Clipboard, db: &ClipboardDb) -> Result<bool, Box<dyn std::error::Error>> {
        if let Ok(image_data) = cb.get_image() {
            let img_hash = Self::calculate_hash(&image_data.bytes);
            let mut last = self.last_hash.lock().unwrap();

            if img_hash != *last {
                log::debug!("ImageExtractor: Hash changed. New content detected.");
                if let Some(img_buffer) = ImageBuffer::<Rgba<u8>, _>::from_raw(
                    image_data.width as u32,
                    image_data.height as u32,
                    image_data.bytes.into_owned(),
                ) {
                    let mut png_bytes: Vec<u8> = Vec::new();
                    let mut cursor = Cursor::new(&mut png_bytes);

                    if img_buffer.write_to(&mut cursor, ImageFormat::Png).is_ok() {
                        db.insert_entry(ContentType::Image, None, Some(&png_bytes))?;

                        *last = img_hash;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    fn try_restore(&self, _db: &ClipboardDb, cb: &mut Clipboard, entry: &HistoryEntry) -> Result<bool, Box<dyn std::error::Error>> {
        if entry.content_type == ContentType::Image {
            if let Some(ref png_bytes) = entry.image_blob {
                let dynamic_img = image::load_from_memory(png_bytes)?;
                let rgba_img = dynamic_img.into_rgba8();
                let (width, height) = rgba_img.dimensions();
                let raw_pixels = rgba_img.into_raw();

                let img_data = ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(raw_pixels),
                };

                cb.set_image(img_data)?;

                let mut last = self.last_hash.lock().unwrap();
                *last = Self::calculate_hash(&entry.image_blob.as_ref().unwrap());

                return Ok(true);
            }
        }
        Ok(false)
    }

    fn clear_memory(&self) {
        let mut last = self.last_hash.lock().unwrap();
        *last = 0;
    }

    fn get_preview(&self, entry: &HistoryEntry) -> String {
        "[🖼️ Image Binary Data]".to_string()
    }
}