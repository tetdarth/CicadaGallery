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
    // Load settings to get saved window size
    let settings = database::load_settings().unwrap_or_default();
    
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("CicadaGallery - Video Gallery Player");
    
    // Apply saved window size or use default
    if let Some((width, height)) = settings.window_size {
        viewport = viewport.with_inner_size([width, height]);
    } else {
        viewport = viewport.with_inner_size([1280.0, 720.0]);
    }
    
    // Apply saved window position
    if let Some((x, y)) = settings.window_position {
        viewport = viewport.with_position([x, y]);
    }
    
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    
    eframe::run_native(
        "CicadaGallery",
        options,
        Box::new(|cc| Ok(Box::new(VideoPlayerApp::new(cc)))),
    )
}
