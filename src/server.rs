use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use crate::database::ClipboardDb;
use crate::types::Command;
use log::{info, error};

pub fn start_server(db: ClipboardDb) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = "/tmp/clip_rust.sock";
    let _ = std::fs::remove_file(socket_path); // Clean up old socket
    let listener = UnixListener::bind(socket_path)?;

    info!("IPC Server listening on {}", socket_path);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = String::new();
                stream.read_to_string(&mut buffer)?;
                let cmd: Command = serde_json::from_str(&buffer)?;

                // Handle the command exactly like your interactive shell did
                match cmd {
                    Command::List => {
                        let items = db.get_pinned_items()?;
                        let response = serde_json::to_string(&items)?;
                        stream.write_all(response.as_bytes())?;
                    }
                    Command::Copy(id) => {
                        crate::clipboard_manager::copy_by_id(&db, id)?;
                    }
                    _ => {}
                }
            }
            Err(e) => error!("Socket error: {}", e),
        }
    }
    Ok(())
}