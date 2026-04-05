use eframe::egui;
use std::io::Write;
use std::os::unix::net::UnixStream;
use crate::database::{ClipboardDb, HistoryEntry};
use crate::formatters::find_deserializer;
use crate::types::{Command, PreviewContent};
use unicode_bidi::BidiInfo;

pub struct GuiItem {
    pub id: i64,
    pub preview: PreviewContent,
}
pub struct ClipboardGui {
    items: Vec<GuiItem>,
    db: ClipboardDb,
    limit: i64,
    offset: i64,
    has_more: bool,
}
impl ClipboardGui {
    pub fn new(cc: &eframe::CreationContext<'_>, db: ClipboardDb) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let limit = 20;
        let mut app = Self {
            items: Vec::new(),
            db,
            limit,
            offset: 0,
            has_more: true,
        };
        app.load_more(); // Load the first batch
        app
    }

    // Bidi helper to supports Hebrew
    fn format_bidi(text: &str) -> String {
        let bidi_info = BidiInfo::new(text, None);
        if bidi_info.paragraphs.is_empty() {
            return text.to_string();
        }
        let para = &bidi_info.paragraphs[0];
        let line = para.range.clone();

        // This flips Hebrew to visual LTR, while keeping English intact
        bidi_info.reorder_line(para, line).into_owned()
    }

    fn load_more(&mut self) {
        if !self.has_more { return; }

        if let Ok(new_entries) = self.db.get_items_paginated(self.limit, self.offset) {
            if (new_entries.len() as i64) < self.limit {
                self.has_more = false;
            }

            for entry in new_entries {
                let extractor = find_deserializer(&entry.content_type);
                let mut preview = extractor.get_preview(&entry);

                // We must format since he also have hebrew
                if let PreviewContent::Text(ref mut text) = preview {
                    *text = Self::format_bidi(text);
                }

                self.items.push(GuiItem {
                    id: entry.id,
                    preview,
                });
            }
            self.offset += self.limit;
        } else {
            self.has_more = false;
        }
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
                        ui.vertical_centered(|ui| {
                            match &item.preview {
                                PreviewContent::Text(text) => {
                                    if ui.button(text).clicked() {
                                        Self::send_command_to_daemon(Command::Copy(item.id));
                                    }
                                }
                                PreviewContent::Image(bytes) => {
                                    let uri = format!("bytes://{}.png", item.id);
                                    let img = egui::Image::from_bytes(uri, bytes.clone()).max_width(150.0);
                                    if ui.add(egui::ImageButton::new(img)).clicked() {
                                        Self::send_command_to_daemon(Command::Copy(item.id));
                                    }
                                }
                            }
                        });
                    });
                }

                if self.has_more {
                    let trigger_rect = ui.allocate_response(
                        egui::vec2(ui.available_width(), 50.0),
                        egui::Sense::hover()
                    );

                    if ui.is_rect_visible(trigger_rect.rect) {
                        self.load_more();
                    }
                    ui.vertical_centered(|ui| ui.spinner());
                }
            });
        });
    }
}