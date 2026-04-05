use arboard::Clipboard;
use std::thread;
use std::time::Duration;
use log::{info, debug, error};

use crate::database::ClipboardDb;
pub fn start_daemon(db: &ClipboardDb) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Clipboard Service in background"); // Changed to info!

    let mut clipboard = Clipboard::new()?;
    let extractors = crate::extractors::get_all();

    loop {
        let mut saved_something = false;

        for extractor in &extractors {
            match extractor.try_save(&mut clipboard, db) {
                Ok(true) => {
                    saved_something = true;
                    break;
                }
                Ok(false) => continue,
                Err(e) => error!("Extractor failed during save: {}", e),
            }
        }

        if saved_something {
            debug!("New content saved. Clearing memory of all formatters to prevent stale states.");
            for extractor in &extractors {
                extractor.clear_memory();
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}

pub fn copy_by_id(db: &ClipboardDb, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    info!("Attempting to restore item ID: {} to OS clipboard", id);
    let entry = db.get_entry(id)?;
    let mut clipboard = Clipboard::new()?;
    let extractors = crate::extractors::get_all();

    for extractor in extractors {
        if extractor.try_restore(db, &mut clipboard, &entry)? {
            info!("Successfully restored item ID: {} using {:?}", id, entry.content_type);
            return Ok(());
        }
    }

    error!("No extractor was able to handle restoration for ID: {}", id);
    Err("No compatible extractor found for this data type".into())
}
