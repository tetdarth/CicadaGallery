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

fn load_icon() -> Option<egui::IconData> {
    let icon_path = std::path::Path::new("image/cicadaGallery.ico");
    if !icon_path.exists() {
        eprintln!("Icon file not found: {:?}", icon_path);
        return None;
    }
    
    // Read ICO file and extract the largest image
    let icon_data = std::fs::read(icon_path).ok()?;
    let icon_dir = ico::IconDir::read(std::io::Cursor::new(&icon_data)).ok()?;
    
    // Decode all entries and find the largest one (width=0 means 256)
    let mut best_image: Option<ico::IconImage> = None;
    let mut best_size: u32 = 0;
    
    for entry in icon_dir.entries() {
        if let Ok(image) = entry.decode() {
            let size = image.width() * image.height();
            if size > best_size {
                best_size = size;
                best_image = Some(image);
            }
        }
    }
    
    let image = best_image?;
    let rgba = image.rgba_data();
    
    eprintln!("Loaded icon: {}x{}", image.width(), image.height());
    
    Some(egui::IconData {
        rgba: rgba.to_vec(),
        width: image.width(),
        height: image.height(),
    })
}

fn main() -> eframe::Result<()> {
    // Load settings to get saved window size
    let settings = database::load_settings().unwrap_or_default();
    
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("CicadaGallery - Video Gallery Player");
    
    // Load and set application icon
    if let Some(icon_data) = load_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon_data));
    }
    
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
