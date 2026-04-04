use crate::database::{ClipboardDb, HistoryEntry};
use crate::types::{ClipboardExtractor, ContentType};
use arboard::Clipboard;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
pub struct TextExtractor {
    last_hash: Mutex<String>,
}

impl TextExtractor {
    pub fn new() -> Self {
        Self {
            last_hash: Mutex::new(String::new()),
        }
    }
}

impl ClipboardExtractor for TextExtractor {
    fn try_save(
        &self,
        cb: &mut Clipboard,
        db: &ClipboardDb,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if let Ok(text) = cb.get_text() {
            if text.trim().is_empty() {
                return Ok(false);
            }

            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            let current_hash = format!("{:x}", hasher.finalize());

            let mut last = self.last_hash.lock().unwrap();
            if *last == current_hash {
                return Ok(true); // Already saved, we don't need to do anything.
            }

            log::debug!("TextExtractor: Hash changed. New content detected.");
            db.insert_entry(ContentType::Text, Some(&text), None)?;
            *last = current_hash;
            return Ok(true);
        }
        Ok(false)
    }

    fn try_restore(
        &self,
        _db: &ClipboardDb,
        cb: &mut Clipboard,
        entry: &HistoryEntry,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if entry.content_type == ContentType::Text {
            if let Some(ref text) = entry.text_content {
                cb.set_text(text.clone())?;
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
        let content = entry.text_content.as_deref().unwrap_or("");
        if content.len() > 30 {
            format!("{}...", &content.chars().take(27).collect::<String>())
        } else {
            content.to_string()
        }
    }
}
