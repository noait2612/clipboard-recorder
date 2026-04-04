use arboard::{Clipboard, Get, Set};
use std::{thread, time};
use std::path::Path;
use bytes::Bytes;
use std::io::{Read};
#[derive(Debug, PartialEq, Eq)]
enum ClipboardFormat {
    Text,
    Png,
    Unknown(String), // For Not supported formats, as in NotInitialized, but I can indicate what was the format
}

fn main() {
    let mut clipboard = Clipboard::new().unwrap();
    let mut previous_text = String::new();
    let mut previous_bytes: Vec<u8> = Vec::new();
    let mut supported_formats = vec![ClipboardFormat::Text, ClipboardFormat::Png]; // Vec to hold GetFormat enums


    // TODO
    // CR
    // BUG
    println!("Listening to clipboard for text, files, and images. Press Ctrl+C to exit.");

    loop {
        let mut current_content = String::new();
        for i in 0..supported_formats.len() {
            if supported_formats[i] == ClipboardFormat::Text  {
                let text = clipboard.get_text().unwrap();
                if text != previous_text {
                    previous_text = text.to_string();
                }
                current_content = text.to_string(); //update current content
            }
            /*
            else if supported_formats[i] == ClipboardFormat::Png {
                let image = clipboard.get_image().unwrap();
                if image.bytes != previous_bytes.as_slice() {
                    previous_bytes = image.bytes.to_vec();
                }
            }
             */
            else {
                println!("Nope");
            }
        }
        thread::sleep(time::Duration::from_millis(100));
        continue;
    }
}