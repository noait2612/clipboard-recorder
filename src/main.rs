mod database;
mod clipboard_manager;
mod types;
mod extractors;

use std::env;
use std::io::{self, Write};
use crate::database::ClipboardDb;
use crate::types::{Command, ContentType};

fn run_interactive_shell(db: &ClipboardDb) {
    println!("Welcome to the Clipboard Interactive Shell!");
    let extractors = crate::extractors::get_all();
    loop {
        print!("clip> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let command = Command::parse(input.trim());
        match command {
            Command::Exit => {
                println!("Closing shell...");
                break;
            }
            Command::Help => {
                println!("Available commands:");
                println!("  list       - Show all pinned items");
                println!("  pin <id>   - Pin an item to your history");
                println!("  unpin <id> - Unpin an item");
                println!("  copy <id>  - Send an item back to the OS clipboard");
                println!("  exit       - Close this shell");
            }
            Command::List => {
                match db.get_pinned_items() {
                    Ok(entries) => {
                        let extractors = crate::extractors::get_all();
                        println!("\n--- 📌 Pinned History ---");

                        if entries.is_empty() {
                            println!("   (No pinned items found)");
                        }

                        for entry in entries {
                            // Use the index helper we discussed or a manual match
                            let idx = entry.content_type.extractor_index();
                            let preview = extractors[idx].get_preview(&entry);

                            println!(
                                "[{}] ({}) {} | Saved: {}",
                                entry.id,
                                entry.content_type.as_str(),
                                preview,
                                database::to_readable_time(entry.created_at)
                            );
                        }
                        println!("-------------------------\n");
                    }
                    Err(e) => eprintln!("Failed to retrieve pinned items: {}", e),
                }
            }
            Command::Pin(id) => {
                let _ = db.set_pin_status(id, true);
            }
            Command::Unpin(id) => {
                let _ = db.set_pin_status(id, false);
            }
            Command::Copy(id) => {
                if let Err(e) = clipboard_manager::copy_by_id(db, id) {
                    eprintln!("Failed to copy: {}", e);
                }
            }
            Command::Clear => {
                print!("Are you sure you want to clear all history? (y/N): ");
                io::stdout().flush().unwrap();
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm).unwrap();

                if confirm.trim().to_lowercase() == "y" {
                    let _ = db.clear_history();
                } else {
                    println!("Clear aborted.");
                }
            }
            Command::Unknown => {
                println!("Unknown command or invalid syntax. Type 'help' for options.");
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("Clipboard service starting up");

    let db = ClipboardDb::new()?;
    let args: Vec<String> = env::args().collect();

    // I do gt and not gt.eq since from some reason args contains a value in the first position by default.
    if args.len() > 1 {
        match args[1].as_str() {
            "shell" => {
                run_interactive_shell(&db);
            }
            "clear" => {
                let _ = db.clear_history();
            }
            _ => {
                println!("I'm lazy maybe I'll implement later...");
            }
        }
    }

    clipboard_manager::start_daemon(&db)?;

    Ok(())
}