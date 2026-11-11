#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod archive;
mod catalog;
mod config;
mod github;
mod inst_ui;
mod inst_update;
mod installers;
mod net;
mod scan;
mod search;
mod state;
mod theme;
mod update;
mod version;
mod winget;

fn app_icon() -> eframe::egui::IconData {
    eframe::egui::IconData {
        rgba: include_bytes!("../assets/icon.rgba").to_vec(),
        width: 128,
        height: 128,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(app::APP_TITLE)
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([760.0, 420.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        app::APP_TITLE,
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(app::LiberApp::new()))
        }),
    )
}
