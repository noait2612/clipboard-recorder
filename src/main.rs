mod database;
mod daemon;
mod types;
mod formatters;
mod gui;

use std::env;
use std::sync::Arc;
use database::ClipboardDb;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "ui" {
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
        log::info!("Starting background daemon");
        let db = Arc::new(ClipboardDb::new()?);
        daemon::run_background_service(db)?;
    }

    Ok(())
}