mod database;
mod types;
mod formatters;
mod gui;
mod service;

use std::env;
use std::sync::Arc;
use database::ClipboardDb;

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
            Box::new(|cc| Box::new(gui::ClipboardGui::new(cc, db))),
        ).unwrap();

    } else {
        log::info!("Starting service");
        let db = Arc::new(ClipboardDb::new()?);
        service::start(db)?;
    }

    Ok(())
}