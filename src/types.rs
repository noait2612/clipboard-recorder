use crate::database::{ClipboardDb, HistoryEntry};
use arboard::Clipboard;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Image,
    File,
}
impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::File => "file",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(ContentType::Text),
            "image" => Some(ContentType::Image),
            "file" => Some(ContentType::File),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    Pin(i64),
    Unpin(i64),
    Copy(i64),
    List,
    ListPinned,
    Help,
    Exit,
    Clear,
    Unknown,
}
pub enum PreviewContent {
    Text(String),
    Image(Vec<u8>),
}

pub trait ClipboardSerializer: Send + Sync {
    fn can_save(&self, cb: &mut Clipboard) -> bool;

    fn save(
        &self,
        cb: &mut Clipboard,
        db: &ClipboardDb,
    ) -> Result<bool, Box<dyn std::error::Error>>;

    fn clear_memory(&self);

    fn seed_memory(&self, cb: &mut Clipboard);
}

pub trait ClipboardDeserializer: Send + Sync {
    fn can_handle(&self, content_type: &ContentType) -> bool;

    fn restore(
        &self,
        cb: &mut Clipboard,
        entry: &HistoryEntry,
    ) -> Result<bool, Box<dyn std::error::Error>>;

    fn get_preview(&self, entry: &HistoryEntry) -> PreviewContent;
}

pub trait ClipboardFormatter: ClipboardSerializer + ClipboardDeserializer + Send + Sync {}
impl<T: ClipboardSerializer + ClipboardDeserializer + Send + Sync> ClipboardFormatter for T {}
