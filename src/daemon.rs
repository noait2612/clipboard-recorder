use crate::database::ClipboardDb;
use crate::formatters::{clear_all_except, find_deserializer, find_serializer};
use crate::types::Command;
use arboard::Clipboard;
use std::io::Read;
use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn run_background_service(db: Arc<ClipboardDb>) -> Result<(), Box<dyn std::error::Error>> {
    let db_watcher = Arc::clone(&db);

    thread::spawn(move || {
        let mut clipboard = Clipboard::new().expect("Failed to open clipboard");

        loop {
            if let Some(serializer) = find_serializer(&mut clipboard) {
                match serializer.save(&mut clipboard, &db_watcher) {
                    Ok(true) => {
                        clear_all_except(serializer);
                    }
                    Ok(false) => {}
                    Err(e) => eprintln!("Saver error: {}", e),
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });

    let socket_path = "/tmp/clip_rust.sock";
    let _ = std::fs::remove_file(socket_path); // Clean up stale socket
    let listener = UnixListener::bind(socket_path)?;

    println!("✅ ClipRust Daemon is active (24/7)");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let db_ref = Arc::clone(&db);
        thread::spawn(move || {
            let mut buffer = String::new();
            if stream.read_to_string(&mut buffer).is_ok() {
                if let Ok(cmd) = serde_json::from_str::<Command>(&buffer) {
                    match cmd {
                        Command::Copy(id) => {
                            if let Ok(entry) = db_ref.get_entry(id) {
                                let mut cb = Clipboard::new().unwrap();
                                let _ = find_deserializer(&entry.content_type).restore(&mut cb, &entry);
                            }
                        }
                        Command::Clear => {
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    Ok(())
}
