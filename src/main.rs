mod database;
mod formatters;
mod gui;
mod service;
mod types;
use database::ClipboardDb;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mode = std::env::var("CLIP_MODE").unwrap_or_else(|_| "service".to_string());
    if mode == "ui" {
        let db = ClipboardDb::new()?;
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([400.0, 600.0])
                .with_always_on_top(),
            ..Default::default()
        };

        eframe::run_native(
            "ClipRust",
            options,
            Box::new(|cc| {
                let mut fonts = eframe::egui::FontDefinitions::default();

                fonts.font_data.insert(
                    "default_font".to_owned(),
                    eframe::egui::FontData::from_static(include_bytes!("assets/DavidLibre-Regular.ttf")),
                );

                fonts.families
                    .get_mut(&eframe::egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "default_font".to_owned());

                // 4. Make it the DEFAULT for all code/monospace text
                fonts.families
                    .get_mut(&eframe::egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "default_font".to_owned());

                // 5. Apply the override to the UI context
                cc.egui_ctx.set_fonts(fonts);

                Box::new(gui::ClipboardGui::new(cc, db))
            }),
        ).unwrap();
    } else {
        log::info!("Starting service");
        let db = Arc::new(ClipboardDb::new()?);
        service::start(db)?;
    }

    Ok(())
}
