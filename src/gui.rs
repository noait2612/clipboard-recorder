use crate::database::ClipboardDb;
use crate::formatters::find_deserializer;
use crate::types::{Command, ContentType, PreviewContent};
use eframe::egui;
use eframe::egui::Ui;
use log::error;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::UnixStream;
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
                    if let Err(e) = stream.write_all(msg.as_bytes()) {
                        log::error!("Failed to write to daemon socket: {}", e);
                    } else {
                        log::debug!("Command {:?} sent to daemon successfully", cmd);
                    }
                }
            }
            Err(e) => {
                log::error!("Could not connect to daemon at {}: {}", socket_path, e);
            }
        }
    }
}

impl eframe::App for ClipboardGui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_pixels_per_point(1.2);
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🔍");

                let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                    let bidi_text = Self::format_bidi(text.as_str());

                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        &bidi_text,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(14.0),
                            color: ui.visuals().text_color(),
                            ..Default::default()
                        },
                    );
                    job.wrap.max_width = wrap_width;

                    ui.fonts_mut(|f| f.layout_job(job))
                };

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .horizontal_align(egui::Align::RIGHT)
                        .layouter(&mut layouter),
                );

                if response.changed() {
                    self.items.clear();
                    self.offset = 0;
                    self.has_more = true;
                    self.load_more();
                }

                if !self.search_query.is_empty() {
                    if ui.button("❌").clicked() {
                        self.search_query.clear();
                        self.items.clear();
                        self.offset = 0;
                        self.has_more = true;
                        self.load_more();
                    }
                }
            });

            ui.separator();

            let load_more_triggered =
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let mut trigger_load = false;

                        for item in &self.items {
                            egui::Frame::default()
                                .inner_margin(egui::Margin::same(4))
                                .corner_radius(4.0)
                                .show(ui, |ui| {
                                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                        match &item.preview {
                                            PreviewContent::Text(text) => {
                                                if ui.button(text).clicked() {
                                                    Self::send_command_to_daemon(Command::Copy(item.id));
                                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                                }
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
                                                        let image = egui::Image::new(texture)
                                                            .maintain_aspect_ratio(true)
                                                            .max_size(egui::vec2(200.0, 200.0))
                                                            .corner_radius(4.0);

                                                        let img_button = egui::Button::image(image)
                                                            .fill(egui::Color32::TRANSPARENT)
                                                            .corner_radius(8.0);
                                                        let response = ui.add_sized([200.0, 200.0], img_button);

                                                        if response.clicked() {
                                                            Self::send_command_to_daemon(Command::Copy(item.id));
                                                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    });
                                });

                            ui.add_space(2.0);
                        }

                        if self.has_more && !self.is_loading {
                            ui.add_space(10.0);
                            ui.add(egui::Spinner::new());
                            ui.add_space(10.0);

                            // Allocate a 1x1 invisible rectangle at the very bottom for the scrolling effect
                            let bottom_rect = ui.allocate_space(egui::vec2(1.0, 1.0)).1;

                            // If this rectangle enters the visible screen area, we've scrolled to the bottom
                            if ui.is_rect_visible(bottom_rect) {
                                trigger_load = true;
                            }
                        }

                        trigger_load
                    });

            if load_more_triggered.inner {
                self.load_more();
            }
        });
    }
}
