#![windows_subsystem = "windows"]

mod models;
mod video_scanner;
mod thumbnail;
mod video_player;
mod database;
mod scene_detection;
mod i18n;
mod license;
mod app;

use app::VideoPlayerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("CicadaGallery - Video Gallery Player"),
        ..Default::default()
    };
    
    eframe::run_native(
        "CicadaGallery",
        options,
        Box::new(|cc| Ok(Box::new(VideoPlayerApp::new(cc)))),
    )
}
