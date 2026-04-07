use crate::database::ClipboardDb;
use crate::types::Command;
use arboard::Clipboard;
use std::io::Read;
use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use crate::formatters::{clear_all_except, find_deserializer, find_serializer, seed_formatters};

pub fn start(db: Arc<ClipboardDb>) -> Result<(), Box<dyn std::error::Error>> {
    let watcher_db = Arc::clone(&db);

    thread::spawn(move || {
        let mut clipboard = Clipboard::new().expect("Clipboard init failed");
        seed_formatters(&mut clipboard);
        loop {
            if let Some(saver) = find_serializer(&mut clipboard) {
                if saver.save(&mut clipboard, &watcher_db).unwrap() {
                    clear_all_except(saver);
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });

    listen_for_commands(db)
}

fn listen_for_commands(db: Arc<ClipboardDb>) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = "/tmp/clip_rust.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    for stream in listener.incoming() {
        let mut stream = stream?;
        let db_ref = Arc::clone(&db);

        thread::spawn(move || {
            let mut buffer = String::new();
            if stream.read_to_string(&mut buffer).is_ok() {
                if let Ok(cmd) = serde_json::from_str::<Command>(&buffer) {
                    handle_command(cmd, db_ref);
                }
            }
        });
    }
    Ok(())
}

fn handle_command(cmd: Command, db: Arc<ClipboardDb>) {
    match cmd {
        Command::Copy(id) => {
            if let Ok(entry) = db.get_entry(id) {
                let mut cb = Clipboard::new().unwrap();
                let deserializer = find_deserializer(&entry.content_type);
                let _ = deserializer.restore(&mut cb, &entry);
            }
        }
        Command::Clear => {
            if let Err(e) = db.clear_history() {
                log::error!("Failed to Clear history");
            }
        },
        Command::TogglePin(id) => {
            if let Err(e) = db.set_pin_status(id) {
                log::error!("Failed to toggle pin for item {}: {}", id, e);
            }
        }
        _ => todo!()
    }
}