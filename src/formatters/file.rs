use crate::database::{ClipboardDb, HistoryEntry};
use crate::types::{ClipboardDeserializer, ClipboardSerializer, ContentType, PreviewContent};
use arboard::Clipboard;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::sync::Mutex;

pub struct FileFormatter {
    last_hash: Mutex<String>,
}

impl FileFormatter {
    pub fn new() -> Self {
        Self {
            last_hash: Mutex::new(String::new()),
        }
    }
}

impl ClipboardDeserializer for FileFormatter {
    fn can_handle(&self, content_type: &ContentType) -> bool {
        matches!(content_type, ContentType::File)
    }

    fn restore(&self, cb: &mut Clipboard, entry: &HistoryEntry) -> Result<bool, Box<dyn Error>> {
        if let Some(ref t) = entry.text_content {
            cb.set_text(t.clone())?;
            return Ok(true);
        }
        Ok(false)
    }

    fn get_preview(&self, entry: &HistoryEntry) -> PreviewContent {
        let path = entry.text_content.as_deref().unwrap_or("");
        PreviewContent::Text(format!("📁 {}", path.split('/').last().unwrap_or(path)))
    }
}

impl ClipboardSerializer for FileFormatter {
    fn can_save(&self, cb: &mut Clipboard) -> bool {
        if let Ok(text) = cb.get_text() {
            return text.starts_with("file://");
        }
        false
    }

    fn save(
        &self,
        cb: &mut Clipboard,
        db: &ClipboardDb,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let text = cb.get_text()?;
        let hash = Sha256::digest(text.as_bytes());
        let current_hash = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        let mut last = self.last_hash.lock().unwrap();
        if *last == current_hash {
            return Ok(true); // Already saved, we don't need to do anything.
        }

        log::debug!("FileExtractor: Hash changed. New content detected.");
        db.insert_entry(ContentType::File, Some(&text), None)?;
        *last = current_hash;
        Ok(true)
    }

    fn clear_memory(&self) {
        let mut last = self.last_hash.lock().unwrap();
        last.clear();
    }
}
