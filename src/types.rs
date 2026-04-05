use crate::database::{ClipboardDb, HistoryEntry};
use arboard::Clipboard;

#[derive(Debug, Clone, PartialEq)]
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

    pub fn extractor_index(&self) -> usize {
        match self {
            ContentType::File => 0,
            ContentType::Image => 1,
            ContentType::Text => 2,
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

#[derive(Debug, Clone, PartialEq)]
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
impl Command {
    pub fn parse(input: &str) -> Self {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Command::Unknown;
        }

        match parts[0] {
            "list" => Command::List,
            "help" => Command::Help,
            "exit" | "quit" => Command::Exit,
            "clear" => Command::Clear,
            "pin" | "unpin" | "copy" => {
                if parts.len() < 2 {
                    return Command::Unknown;
                }
                if let Ok(id) = parts[1].parse::<i64>() {
                    match parts[0] {
                        "pin" => Command::Pin(id),
                        "unpin" => Command::Unpin(id),
                        "copy" => Command::Copy(id),
                        _ => Command::Unknown,
                    }
                } else {
                    Command::Unknown
                }
            }
            _ => Command::Unknown,
        }
    }
}
pub trait ClipboardExtractor {
    fn can_handle(&self, content_type: &ContentType) -> bool;

    fn try_save(
        &self,
        cb: &mut Clipboard,
        db: &ClipboardDb,
    ) -> Result<bool, Box<dyn std::error::Error>>;

    fn try_restore(
        &self,
        db: &ClipboardDb,
        cb: &mut Clipboard,
        entry: &HistoryEntry,
    ) -> Result<bool, Box<dyn std::error::Error>>;
    fn clear_memory(&self);

    fn get_preview(&self, entry: &HistoryEntry) -> String;
}
