use arboard::{Clipboard, ImageData};
use chrono::Local;
use image::ImageBuffer;
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::thread;
use std::time::Duration;

use crate::database::ClipboardDb;
use crate::types::ContentType;

const TIME_FORMAT: &str = "%m-%d-%Y %H:%M:%S";
fn get_timestamp() -> String {
    let timestamp = Local::now().format(TIME_FORMAT).to_string();
    timestamp
}
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn start_daemon(db: &ClipboardDb) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Clipboard Service in background");

    let mut clipboard = Clipboard::new()?;
    let mut last_copied_text = String::new();
    let mut last_copied_image_hash: u64 = 0;

    loop {
        if let Ok(current_text) = clipboard.get_text() {
            if current_text != last_copied_text && !current_text.trim().is_empty() {
                println!("New text copy detected.");

                let content_type = if current_text.starts_with("file://") {
                    ContentType::File
                } else {
                    ContentType::Text
                };
                db.insert_entry(content_type, Some(&current_text), None, &get_timestamp())?;
                last_copied_text = current_text.clone();
                last_copied_image_hash = 0;
            }
        }
        else if let Ok(image_data) = clipboard.get_image() {
            let img_hash = calculate_hash(&image_data.bytes);

            if img_hash != last_copied_image_hash {
                println!("New image copy detected! Size: {}x{}", image_data.width, image_data.height);

                if let Some(img_buffer) = ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                    image_data.width as u32,
                    image_data.height as u32,
                    image_data.bytes.into_owned(),
                ) {
                    let mut png_bytes: Vec<u8> = Vec::new();
                    let mut cursor = Cursor::new(&mut png_bytes);

                    if img_buffer.write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
                        let timestamp = Local::now().format(TIME_FORMAT).to_string();
                        db.insert_entry(ContentType::Image, None, Some(&png_bytes), &timestamp)?;
                        println!("Successfully saved image to database.");
                    }
                }

                last_copied_image_hash = img_hash;
                last_copied_text.clear();
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}

pub fn copy_by_id(db: &ClipboardDb, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let entry = db.get_entry(id)?;
    let mut clipboard = Clipboard::new()?;

    match entry.content_type {
        ContentType::Text | ContentType::File => {
            if let Some(text) = entry.text_content {
                clipboard.set_text(text)?;
                println!("Successfully restored {:?} (ID: {}) to clipboard.", entry.content_type, id);
            } else {
                return Err("Database error: Text content is missing for a text/file entry.".into());
            }
        }

        ContentType::Image => {
            if let Some(png_bytes) = entry.image_blob {
                // 1. Decode the compressed PNG bytes from the DB back into a DynamicImage
                let dynamic_img = image::load_from_memory(&png_bytes)?;

                // 2. Convert it back to raw 8-bit RGBA pixels
                let rgba_img = dynamic_img.into_rgba8();
                let (width, height) = rgba_img.dimensions();

                // 3. Extract the raw pixel vector
                let raw_pixels = rgba_img.into_raw();

                // 4. Wrap it in arboard's ImageData struct
                let img_data = ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(raw_pixels),
                };

                // 5. Push to OS Clipboard
                clipboard.set_image(img_data)?;
                println!("Successfully restored Image (ID: {}) to clipboard.", id);
            } else {
                return Err("Database error: Image blob is missing for an image entry.".into());
            }
        }
    }

    Ok(())
}