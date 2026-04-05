mod file;
mod image;
mod text;
use crate::formatters::file::FileFormatter;
use crate::formatters::image::ImageFormatter;
use crate::formatters::text::TextFormatter;
use crate::types::{ClipboardDeserializer, ClipboardFormatter, ClipboardSerializer, ContentType};
use arboard::Clipboard;
use std::sync::OnceLock;
static REGISTRY: OnceLock<Vec<Box<dyn ClipboardFormatter>>> = OnceLock::new();
fn get_registry() -> &'static Vec<Box<dyn ClipboardFormatter>> {
    REGISTRY.get_or_init(|| {
        vec![
            Box::new(FileFormatter::new()),
            Box::new(ImageFormatter::new()),
            Box::new(TextFormatter::new()),
        ]
    })
}

pub fn seed_formatters(cb: &mut Clipboard) {
    let registry = get_registry();
    for ext in registry {
        ext.seed_memory(cb);
    }
}

pub fn find_serializer(cb: &mut Clipboard) -> Option<&'static dyn ClipboardSerializer> {
    get_registry()
        .iter()
        .find(|ext| ext.can_save(cb))
        .map(|ext| &**ext as &dyn ClipboardSerializer)
}

pub fn find_deserializer(ct: &ContentType) -> &'static dyn ClipboardDeserializer {
    get_registry()
        .iter()
        .find(|ext| ext.can_handle(ct))
        .map(|ext| &**ext as &dyn ClipboardDeserializer)
        .expect("Unsupported content type")
}

pub fn clear_all_except(active: &dyn ClipboardSerializer) {
    let registry = get_registry();
    for ext in registry {
        let saver = &**ext as &dyn ClipboardSerializer;
        if !std::ptr::eq(saver, active) {
            saver.clear_memory();
        }
    }
}
