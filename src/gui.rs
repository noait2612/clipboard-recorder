use eframe::egui;
use std::io::Write;
use std::os::unix::net::UnixStream;
use crate::database::{ClipboardDb, HistoryEntry};
use crate::formatters::find_deserializer;
use crate::types::{Command, PreviewContent};

pub struct ClipboardGui {
    items: Vec<HistoryEntry>,
}

impl ClipboardGui {
    pub fn new(cc: &eframe::CreationContext<'_>, db: ClipboardDb) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        // Load the first page of history on startup
        let items = db.get_ordered_items().unwrap_or_default();
        Self { items }
    }

    fn send_command_to_daemon(cmd: Command) {
        if let Ok(mut stream) = UnixStream::connect("/tmp/clip_rust.sock") {
            if let Ok(msg) = serde_json::to_string(&cmd) {
                let _ = stream.write_all(msg.as_bytes());
            }
        }
    }
}

impl eframe::App for ClipboardGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📋 Clipboard History");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for item in &self.items {
                    ui.group(|ui| {
                        let extractor = find_deserializer(&item.content_type);
                        let preview = extractor.get_preview(item);

                        ui.horizontal(|ui| {
                            match preview {
                                PreviewContent::Text(text) => {
                                    if ui.button(text).clicked() {
                                        Self::send_command_to_daemon(Command::Copy(item.id));
                                        std::process::exit(0); // Close GUI on copy
                                    }
                                }
                                PreviewContent::Image(bytes) => {
                                    let uri = format!("bytes://{}.png", item.id);
                                    let img = egui::Image::from_bytes(uri, bytes).max_width(150.0);
                                    if ui.add(egui::ImageButton::new(img)).clicked() {
                                        Self::send_command_to_daemon(Command::Copy(item.id));
                                        std::process::exit(0);
                                    }
                                }
                            }
                        });
                    });
                }
            });
        });
    }
}