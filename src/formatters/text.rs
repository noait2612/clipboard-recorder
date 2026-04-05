use crate::database::{ClipboardDb, HistoryEntry};
use crate::types::{ClipboardDeserializer, ClipboardSerializer, ContentType, PreviewContent};
use arboard::Clipboard;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::sync::Mutex;
pub struct TextFormatter {
    last_hash: Mutex<String>,
}

impl TextFormatter {
    pub fn new() -> Self {
        Self {
            last_hash: Mutex::new(String::new()),
        }
    }
}
impl ClipboardDeserializer for TextFormatter {
    fn can_handle(&self, content_type: &ContentType) -> bool {
        matches!(content_type, ContentType::Text)
    }

    fn restore(&self, cb: &mut Clipboard, entry: &HistoryEntry) -> Result<bool, Box<dyn Error>> {
        if entry.content_type == ContentType::Text {
            if let Some(ref text) = entry.text_content {
                cb.set_text(text.clone())?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn get_preview(&self, entry: &HistoryEntry) -> PreviewContent {
        let content = entry.text_content.as_deref().unwrap_or("");
        let preview = if content.len() > 80 {
            format!("{}...", &content.chars().take(77).collect::<String>())
        } else {
            content.to_string()
        };
        PreviewContent::Text(preview)
    }
}
impl ClipboardSerializer for TextFormatter {
    fn can_save(&self, cb: &mut Clipboard) -> bool {
        if let Ok(text) = cb.get_text() {
            return !text.starts_with("file://");
        }
        false
    }

    fn save(
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

    fn clear_memory(&self) {
        let mut last = self.last_hash.lock().unwrap();
        last.clear();
    }
}
