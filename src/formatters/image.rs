use crate::database::{ClipboardDb, HistoryEntry};
use crate::types::{ClipboardDeserializer, ClipboardSerializer, ContentType, PreviewContent};
use arboard::{Clipboard, ImageData};
use image::{ImageBuffer, ImageFormat, Rgba};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::sync::Mutex;

pub struct ImageFormatter {
    last_hash: Mutex<u64>,
}

impl ImageFormatter {
    pub fn new() -> Self {
        Self {
            last_hash: Mutex::new(0),
        }
    }

    fn calculate_hash(bytes: &[u8]) -> u64 {
        let mut s = DefaultHasher::new();
        bytes.hash(&mut s);
        s.finish()
    }

    fn generate_thumbnail(&self, full_png_bytes: &[u8]) -> Option<Vec<u8>> {
        if let Ok(dynamic_img) = image::load_from_memory(full_png_bytes) {
            let thumb = dynamic_img.thumbnail(150, 150);
            let mut out_bytes = Cursor::new(Vec::new());
            if thumb.write_to(&mut out_bytes, ImageFormat::Png).is_ok() {
                return Some(out_bytes.into_inner());
            }
        }
        None
    }
}

impl ClipboardDeserializer for ImageFormatter {
    fn can_handle(&self, content_type: &ContentType) -> bool {
        matches!(content_type, ContentType::Image)
    }

    fn restore(&self, cb: &mut Clipboard, entry: &HistoryEntry) -> Result<bool, Box<dyn Error>> {
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

    fn get_preview(&self, entry: &HistoryEntry) -> PreviewContent {
        if let Some(ref full_bytes) = entry.image_blob {
            if let Some(thumb_bytes) = self.generate_thumbnail(full_bytes) {
                return PreviewContent::Image(thumb_bytes);
            }
        }
        PreviewContent::Text("[Image Rendering Error]".to_string())
    }
}
impl ClipboardSerializer for ImageFormatter {
    fn can_save(&self, cb: &mut Clipboard) -> bool {
        if let Ok(_image) = cb.get_image() {
            return true;
        }
        false
    }

    fn save(
        &self,
        cb: &mut Clipboard,
        db: &ClipboardDb,
    ) -> Result<bool, Box<dyn std::error::Error>> {
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

    fn clear_memory(&self) {
        let mut last = self.last_hash.lock().unwrap();
        *last = 0;
    }

    fn seed_memory(&self, cb: &mut Clipboard) {
        if self.can_save(cb) {
            let img_hash = Self::calculate_hash(&cb.get_image().unwrap().bytes);
            let mut last = self.last_hash.lock().unwrap();
            *last = img_hash;
            log::debug!("ImageFormatter: Memory seeded with current clipboard.");
        }
    }
}
