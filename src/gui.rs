use crate::database::ClipboardDb;
use crate::formatters::find_deserializer;
use crate::types::{Command, PreviewContent};
use eframe::egui;
use eframe::egui::Ui;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::UnixStream;
use unicode_bidi::BidiInfo;

pub struct GuiItem {
    pub id: i64,
    pub preview: PreviewContent,
    pub is_pinned: bool,
}

pub struct ClipboardGui {
    items: Vec<GuiItem>,
    db: ClipboardDb,
    limit: i64,
    offset: i64,
    has_more: bool,
    search_query: String,
    is_loading: bool,
    textures: HashMap<i64, egui::TextureHandle>,
}

impl ClipboardGui {
    pub fn new(cc: &eframe::CreationContext<'_>, db: ClipboardDb) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let limit = 10;
        let mut app = Self {
            items: Vec::new(),
            db,
            limit,
            offset: 0,
            has_more: true,
            search_query: String::new(),
            is_loading: false,
            textures: HashMap::new(),
        };

        app.load_more();
        app
    }

    fn format_bidi(text: &str) -> String {
        let bidi_info = BidiInfo::new(text, None);
        if bidi_info.paragraphs.is_empty() {
            return text.to_string();
        }
        let para = &bidi_info.paragraphs[0];
        let line = para.range.clone();
        bidi_info.reorder_line(para, line).into_owned()
    }

    fn load_more(&mut self) {
        if !self.has_more {
            return;
        }

        self.is_loading = true;

        let result = if self.search_query.trim().is_empty() {
            self.db.get_items_paginated(self.limit, self.offset)
        } else {
            self.db
                .search_items_paginated(self.search_query.trim(), self.limit, self.offset)
        };

        if let Ok(new_entries) = result {
            if (new_entries.len() as i64) < self.limit {
                self.has_more = false;
            }

            for entry in new_entries {
                let extractor = find_deserializer(&entry.content_type);
                let mut preview = extractor.get_preview(&entry);

                if let PreviewContent::Text(ref mut text) = preview {
                    *text = Self::format_bidi(text);
                }

                self.items.push(GuiItem {
                    id: entry.id,
                    preview,
                    is_pinned: entry.is_pinned,
                });
            }

            self.offset += self.limit;
        } else {
            self.has_more = false;
        }

        self.is_loading = false;
    }

    fn send_command_to_daemon(cmd: Command) {
        let socket_path = "/tmp/clip_rust.sock";
        match UnixStream::connect(socket_path) {
            Ok(mut stream) => {
                if let Ok(msg) = serde_json::to_string(&cmd) {
                    let _ = stream.write_all(msg.as_bytes());
                }
            }
            Err(e) => log::error!("Could not connect to daemon: {}", e),
        }
    }

    fn clear_history(&mut self) {
        Self::send_command_to_daemon(Command::Clear);
        self.items.clear();
        self.textures.clear();
        self.offset = 0;
        self.has_more = true;
        self.load_more();
    }
}

impl eframe::App for ClipboardGui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_pixels_per_point(1.2);

        // Theme Colors
        let bg_color = egui::Color32::from_rgb(27, 27, 27);
        let hover_overlay = egui::Color32::from_rgba_premultiplied(50, 50, 50, 10);
        let text_color = egui::Color32::from_rgb(180, 180, 180);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("🔍").color(text_color));

                let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                    let bidi_text = Self::format_bidi(text.as_str());
                    let mut job = egui::text::LayoutJob::default();
                    job.halign = egui::Align::RIGHT;
                    job.append(
                        &bidi_text,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(16.0),
                            color: text_color,
                            ..Default::default()
                        },
                    );
                    job.wrap.max_width = wrap_width;
                    ui.fonts_mut(|f| f.layout_job(job))
                };

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search...")
                        .layouter(&mut layouter),
                );

                if response.changed() {
                    self.items.clear();
                    self.offset = 0;
                    self.has_more = true;
                    self.load_more();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("🗑").color(text_color)).clicked() {
                        self.clear_history();
                    }
                    if !self.search_query.is_empty() {
                        if ui.button(egui::RichText::new("❌").color(text_color)).clicked() {
                            self.search_query.clear();
                            self.items.clear();
                            self.offset = 0;
                            self.has_more = true;
                            self.load_more();
                        }
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();

            let mut toggle_pin_id = None;

            let load_more_triggered = egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut trigger_load = false;

                    for item in &self.items {
                        let row_width = ui.available_width();
                        let row_id = ui.make_persistent_id(item.id);

                        // Paint Frame
                        let frame_res = egui::Frame::default()
                            .inner_margin(egui::Margin::same(4))
                            .fill(bg_color)
                            .show(ui, |ui| {
                                ui.set_width(row_width);
                                ui.horizontal(|ui| {
                                    if item.is_pinned {
                                        ui.label(egui::RichText::new("📌").size(8.0));
                                    }
                                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                        match &item.preview {
                                            PreviewContent::Text(text) => {
                                                ui.label(egui::RichText::new(text).color(text_color));
                                            }
                                            PreviewContent::Image(bytes) => {
                                                ui.push_id(item.id, |ui| {
                                                    if !self.textures.contains_key(&item.id) {
                                                        if let Ok(image) = image::load_from_memory(bytes) {
                                                            let image = image.to_rgba8();
                                                            let size = [image.width() as usize, image.height() as usize];
                                                            let pixels = image.into_vec();
                                                            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                                                            let texture = ui.ctx().load_texture(
                                                                format!("clipboard_img_{}", item.id),
                                                                color_image,
                                                                egui::TextureOptions::default(),
                                                            );
                                                            self.textures.insert(item.id, texture);
                                                        }
                                                    }
                                                    if let Some(texture) = self.textures.get(&item.id) {
                                                        ui.add(egui::Image::new(texture)
                                                            .maintain_aspect_ratio(true)
                                                            .max_size(egui::vec2(row_width * 0.8, 300.0)));
                                                    }
                                                });
                                            }
                                        }
                                    });
                                });
                            });

                        let row_interact = ui.interact(frame_res.response.rect, row_id, egui::Sense::click());
                        if row_interact.hovered() {
                            ui.painter().rect_filled(frame_res.response.rect, 0.0, hover_overlay);
                        }
                        if row_interact.clicked() {
                            Self::send_command_to_daemon(Command::Copy(item.id));
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        row_interact.context_menu(|ui| {
                            let label = if item.is_pinned { "Unpin" } else { "Pin" };
                            if ui.button(label).clicked() {
                                toggle_pin_id = Some(item.id);
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });

                        ui.add_space(6.0);
                    }

                    if self.has_more && !self.is_loading {
                        let bottom_rect = ui.allocate_space(egui::vec2(1.0, 1.0)).1;
                        if ui.is_rect_visible(bottom_rect) { trigger_load = true; }
                    }
                    trigger_load
                }).inner;

            if let Some(id) = toggle_pin_id {
                Self::send_command_to_daemon(Command::TogglePin(id));
                self.items.clear();
                self.offset = 0;
                self.has_more = true;
                self.load_more();
            }

            if load_more_triggered {
                self.load_more();
            }
        });
    }
}
