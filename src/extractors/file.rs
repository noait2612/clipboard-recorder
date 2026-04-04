use crate::database::{ClipboardDb, HistoryEntry};
use crate::types::{ClipboardExtractor, ContentType};
use arboard::Clipboard;
use sha2::{Digest, Sha256};
use std::sync::Mutex;

pub struct FileExtractor {
    last_hash: Mutex<String>,
}

impl FileExtractor {
    pub fn new() -> Self {
        Self {
            last_hash: Mutex::new(String::new()),
        }
    }
}

impl ClipboardExtractor for FileExtractor {
    fn try_save(
        &self,
        cb: &mut Clipboard,
        db: &ClipboardDb,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if let Ok(text) = cb.get_text() {
            if text.starts_with("file://") {
                let mut hasher = Sha256::new();
                hasher.update(text.as_bytes());
                let current_hash = format!("{:x}", hasher.finalize());
                let mut last = self.last_hash.lock().unwrap();
                if *last == current_hash {
                    return Ok(true); // Already saved, we don't need to do anything.
                }

                log::debug!("FileExtractor: Hash changed. New content detected.");
                db.insert_entry(ContentType::File, Some(&text), None)?;
                *last = current_hash;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn try_restore(
        &self,
        _db: &ClipboardDb,
        cb: &mut Clipboard,
        entry: &HistoryEntry,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if entry.content_type == ContentType::File {
            if let Some(ref path) = entry.text_content {
                cb.set_text(path.clone())?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn clear_memory(&self) {
        let mut last = self.last_hash.lock().unwrap();
        last.clear();
    }

    fn get_preview(&self, entry: &HistoryEntry) -> String {
        let path = entry.text_content.as_deref().unwrap_or("Unknown");
        let filename = path.split('/').last().unwrap_or(path);
        format!("📁 File: {}", filename)
    }
}
