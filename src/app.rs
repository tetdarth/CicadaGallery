use crate::models::{VideoDatabase, VideoFile};
use crate::video_scanner;
use crate::video_player;
use crate::database;
use crate::scene_detection::{self, get_video_duration, get_video_resolution};
use crate::thumbnail;
use crate::i18n::{I18n, Language};
use crate::license;
use eframe::egui;
use rfd::FileDialog;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use notify::{Watcher, RecursiveMode, Event};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::SystemTime;

pub struct VideoPlayerApp {
    pub database: VideoDatabase,
    pub selected_video: Option<String>,
    pub current_view: ViewMode,
    pub search_query: String,
    pub selected_folder_filter: HashSet<String>, // Multiple folder selection (premium feature)
    pub selected_tag_filter: HashSet<String>, // Multiple tag selection (premium feature)
    pub last_selected_folder: Option<usize>, // Last selected folder index for shift-click range selection
    pub last_selected_tag: Option<usize>, // Last selected tag index for shift-click range selection
    pub tag_filter_mode: FilterMode, // AND or OR mode for tag filter (premium feature)
    pub min_rating_filter: u8, // 0 = show all, 1-5 = show videos with rating >= this value
    pub show_options_window: bool,
    pub show_folder_management_window: bool, // Show folder management window
    pub show_tag_management_window: bool, // Show tag management window
    pub show_shader_management_window: bool, // Show shader management window
    pub show_mpv_shortcuts: bool, // Show mpv keyboard shortcuts panel
    pub mpv_shortcuts_open: bool, // MPV shortcuts panel open/collapsed state
    pub mpv_shortcuts_pos: Option<egui::Pos2>, // MPV shortcuts panel position
    pub thumbnail_scale: f32, // 0.5 to 3.0 (50% to 300%)
    pub scene_panel_visible: bool, // Show/hide scene panel
    pub texture_cache: HashMap<PathBuf, egui::TextureHandle>,
    pub favorite_icon_texture: Option<egui::TextureHandle>,
    pub delete_confirm_video: Option<String>, // Video ID pending deletion confirmation
    pub selected_videos: HashSet<String>, // IDs of selected videos for multi-selection
    pub last_selected_video: Option<String>, // Last selected video ID for shift-click range selection
    pub mpv_always_on_top: bool, // Keep mpv window always on top
    pub mpv_volume: u8, // MPV volume (0-100)
    pub selected_scenes: HashSet<usize>, // Indices of selected scenes
    pub last_selected_scene: Option<usize>, // Last selected scene index for shift-click range selection
    pub show_tag_add_popup: bool, // Show tag addition popup
    pub new_tag_input: String, // Input for new tag creation
    pub new_folder_input: String, // Input for new folder creation
    pub show_full_filename: bool, // Show full filename in grid view
    pub show_tags_in_grid: bool, // Show tags in grid view
    pub dark_mode: bool, // Dark mode theme
    pub use_gpu_hq: bool, // Use GPU high-quality rendering (mpv profile=gpu-hq)
    pub use_custom_shaders: bool, // Use custom GLSL shaders from mpv/glsl_shaders directory
    pub selected_shader: Option<String>, // Selected shader filename
    pub use_frame_interpolation: bool, // Use frame interpolation (motion smoothing)
    pub i18n: I18n, // Internationalization
    pub metadata_loaded: HashSet<String>, // Videos that have completed metadata loading
    pub sort_field: SortField, // Current sort field
    pub sort_order: SortOrder, // Current sort order
    pub watched_folders: HashSet<PathBuf>, // Folders being watched for changes
    pub fs_watcher: Option<notify::RecommendedWatcher>, // File system watcher
    pub fs_events: Option<Arc<Mutex<Receiver<Result<Event, notify::Error>>>>>, // Channel for file system events
    pub pending_rescan: bool, // Flag to trigger rescan on next update
    pub last_rescan_time: SystemTime, // Last time a rescan was performed
    pub is_premium: bool, // Premium/free tier (false = free, true = premium)
    pub show_premium_promotion_window: bool, // Show premium promotion window when limit reached
    pub show_license_window: bool, // Show license activation window
    pub license_input: String, // License key input field
    pub license_status_message: Option<String>, // License activation status message
    pub current_license: Option<license::License>, // Currently activated license
    pub folder_delete_confirm: Option<(String, usize)>, // (folder_name, video_count) pending deletion
    pub tag_delete_confirm: Option<(String, usize)>, // (tag_name, video_count) pending deletion
    pub last_window_size: Option<(f32, f32)>, // Track last window size for saving on exit
    pub last_window_pos: Option<(f32, f32)>, // Track last window position
    // Async thumbnail loading
    pub pending_textures: HashSet<PathBuf>, // Textures currently being loaded
    pub failed_textures: HashSet<PathBuf>, // Textures that failed to load (to avoid retrying)
    pub texture_load_receiver: Option<Receiver<(PathBuf, Vec<u8>, [usize; 2])>>, // Receiver for loaded image data
    pub texture_load_sender: Option<Sender<(PathBuf, Vec<u8>, [usize; 2])>>, // Sender for loaded image data
    pub texture_fail_receiver: Option<Receiver<PathBuf>>, // Receiver for failed image paths
    pub texture_fail_sender: Option<Sender<PathBuf>>, // Sender for failed image paths
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Grid,
    List,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortField {
    AddedDate,
    FileName,
    Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum FilterMode {
    Or,  // Any of the selected (OR)
    And, // All of the selected (AND)
}

impl Default for VideoPlayerApp {
    fn default() -> Self {
        // Load database
        let mut database = database::load_database().unwrap_or_else(|_| VideoDatabase::new());
        
        // Load settings
        let mut settings = database::load_settings().unwrap_or_default();
        
        // Update added_date from file metadata for all existing videos (only once)
        if !settings.added_dates_updated && !database.videos.is_empty() {
            eprintln!("[init] Updating added_date for all videos from file metadata");
            database.update_added_dates_from_files();
            let _ = database::save_database(&database);
            
            // Mark as updated
            settings.added_dates_updated = true;
            let _ = database::save_settings(&settings);
        }
        
        // Remove duplicate videos (only once per session, at startup)
        let duplicate_count = database.remove_duplicates();
        if duplicate_count > 0 {
            eprintln!("[init] Removed {} duplicate videos", duplicate_count);
            let _ = database::save_database(&database);
        }
        
        // Initialize i18n with loaded language
        let i18n = I18n::new(settings.language);
        
        // Verify license if exists (only source of premium status)
        let (is_premium, current_license) = if let Some(ref license_key) = settings.license_key {
            match license::verify_license(license_key) {
                Ok(license) if license.is_premium_active() => {
                    eprintln!("[License] Valid premium license loaded for {}", license.info.issued_to);
                    (true, Some(license))
                },
                Ok(license) if license.is_expired => {
                    eprintln!("[License] License expired on {}", 
                        license.info.expires_at.map(|ts| license::format_timestamp(ts))
                            .unwrap_or_else(|| "Unknown".to_string()));
                    (false, Some(license))
                },
                Ok(license) => {
                    eprintln!("[License] Invalid license type: {}", license.info.license_type);
                    (false, Some(license))
                },
                Err(e) => {
                    eprintln!("[License] Failed to verify license: {}", e);
                    (false, None)
                }
            }
        } else {
            eprintln!("[License] No license key found - running in free mode");
            (false, None)
        };
        
        Self {
            database,
            selected_video: None,
            current_view: ViewMode::Grid,
            search_query: String::new(),
            selected_folder_filter: HashSet::new(),
            selected_tag_filter: HashSet::new(),
            last_selected_folder: None,
            last_selected_tag: None,
            tag_filter_mode: FilterMode::Or,
            min_rating_filter: 0,
            show_options_window: false,
            show_folder_management_window: false,
            show_tag_management_window: false,
            show_shader_management_window: false,
            show_mpv_shortcuts: true,
            mpv_shortcuts_open: settings.mpv_shortcuts_open,
            mpv_shortcuts_pos: settings.mpv_shortcuts_position.map(|(x, y)| egui::Pos2::new(x, y)),
            thumbnail_scale: settings.thumbnail_scale,
            scene_panel_visible: true,
            texture_cache: HashMap::new(),
            favorite_icon_texture: None,
            delete_confirm_video: None,
            selected_videos: HashSet::new(),
            last_selected_video: None,
            mpv_always_on_top: settings.mpv_always_on_top,
            mpv_volume: settings.mpv_volume,
            selected_scenes: HashSet::new(),
            last_selected_scene: None,
            show_tag_add_popup: false,
            new_tag_input: String::new(),
            new_folder_input: String::new(),
            show_full_filename: settings.show_full_filename,
            show_tags_in_grid: settings.show_tags_in_grid,
            dark_mode: settings.dark_mode,
            use_gpu_hq: settings.use_gpu_hq,
            use_custom_shaders: settings.use_custom_shaders,
            selected_shader: settings.selected_shader,
            use_frame_interpolation: settings.use_frame_interpolation,
            i18n,
            metadata_loaded: HashSet::new(),
            sort_field: SortField::AddedDate,
            sort_order: SortOrder::Descending,
            watched_folders: settings.watched_folders.into_iter().collect(),
            fs_watcher: None,
            fs_events: None,
            pending_rescan: false,
            last_rescan_time: SystemTime::now(),
            is_premium,
            show_premium_promotion_window: false,
            show_license_window: false,
            license_input: String::new(),
            license_status_message: None,
            current_license,
            folder_delete_confirm: None,
            tag_delete_confirm: None,
            last_window_size: None,
            last_window_pos: None,
            // Async thumbnail loading
            pending_textures: HashSet::new(),
            failed_textures: HashSet::new(),
            texture_load_receiver: None,
            texture_load_sender: None,
            texture_fail_receiver: None,
            texture_fail_sender: None,
        }
    }
}

impl VideoPlayerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set up custom fonts for multilingual support (especially Japanese)
        let mut fonts = egui::FontDefinitions::default();
        
        // Try to load Windows system fonts that support Japanese and other languages
        // Try multiple fonts in order of preference
        let font_paths = vec![
            ("meiryo", "C:\\Windows\\Fonts\\meiryo.ttc"),      // Meiryo (Japanese)
            ("yugo", "C:\\Windows\\Fonts\\YuGothM.ttc"),        // Yu Gothic (Japanese)
            ("msgothic", "C:\\Windows\\Fonts\\msgothic.ttc"),  // MS Gothic (Japanese)
            ("segoeui", "C:\\Windows\\Fonts\\segoeui.ttf"),    // Segoe UI (supports many languages)
        ];
        
        let mut loaded_fonts = Vec::new();
        
        for (font_name, font_path) in font_paths {
            if let Ok(font_data) = std::fs::read(font_path) {
                fonts.font_data.insert(
                    font_name.to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                loaded_fonts.push(font_name.to_owned());
            }
        }
        
        // Set font families priorities - use loaded fonts first, then default fonts
        if !loaded_fonts.is_empty() {
            let proportional = fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default();
            
            // Insert loaded fonts at the beginning
            for (i, font_name) in loaded_fonts.iter().enumerate() {
                proportional.insert(i, font_name.clone());
            }
        }
        
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        
        let app = Self::default();
        
        app
    }
    
    /// Save current settings to file
    pub fn save_settings(&self) {
        // Extract license key from current license or settings
        let license_key = self.current_license.as_ref()
            .and_then(|_| {
                // Get from loaded settings
                database::load_settings().ok()
                    .and_then(|s| s.license_key)
            });
        
        let settings = crate::models::AppSettings {
            thumbnail_scale: self.thumbnail_scale,
            mpv_always_on_top: self.mpv_always_on_top,
            show_full_filename: self.show_full_filename,
            show_tags_in_grid: self.show_tags_in_grid,
            dark_mode: self.dark_mode,
            use_gpu_hq: self.use_gpu_hq,
            use_custom_shaders: self.use_custom_shaders,
            selected_shader: self.selected_shader.clone(),
            use_frame_interpolation: self.use_frame_interpolation,
            language: self.i18n.get_language(),
            added_dates_updated: true, // Keep as true to avoid re-updating
            watched_folders: self.watched_folders.iter().cloned().collect(),
            mpv_shortcuts_open: self.mpv_shortcuts_open,
            mpv_shortcuts_position: self.mpv_shortcuts_pos.map(|p| (p.x, p.y)),
            mpv_volume: self.mpv_volume,
            license_key,
            window_size: self.last_window_size,
            window_position: self.last_window_pos,
            window_maximized: false, // Don't save maximized state to avoid flicker on startup
        };
        
        if let Err(e) = database::save_settings(&settings) {
            eprintln!("Failed to save settings: {}", e);
        }
    }
    
    // Execute methods for heavy operations
    pub fn add_files(&mut self) {
        // Check video limit for free tier
        if !self.is_premium && self.database.videos.len() >= 100 {
            eprintln!("[Free tier] Video limit reached (100 videos max). Upgrade to premium for unlimited videos.");
            self.show_premium_promotion_window = true;
            return;
        }
        
        if let Some(files) = FileDialog::new()
            .add_filter("Video Files", &["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg"])
            .pick_files()
        {
            let cache_dir = thumbnail::get_cache_dir();
            for file in files {
                // Check limit for each file
                if !self.is_premium && self.database.videos.len() >= 100 {
                    eprintln!("[Free tier] Video limit reached. Skipping remaining files.");
                    self.show_premium_promotion_window = true;
                    break;
                }
                
                let mut video = VideoFile::new(file.clone());
                
                // Generate thumbnail
                video.thumbnail_path = thumbnail::create_video_thumbnail(&file, &cache_dir);
                
                // Get video metadata using FFmpeg
                video.duration = get_video_duration(&file);
                video.resolution = get_video_resolution(&file);
                
                self.database.add_video(video);
            }
            // Save database
            let _ = database::save_database(&self.database);
        }
    }
    
    pub fn add_folder(&mut self) {
        // Check video limit for free tier
        if !self.is_premium && self.database.videos.len() >= 100 {
            eprintln!("[Free tier] Video limit reached (100 videos max). Upgrade to premium for unlimited videos.");
            self.show_premium_promotion_window = true;
            return;
        }
        
        if let Some(folder) = FileDialog::new().pick_folder() {
            let cache_dir = thumbnail::get_cache_dir();
            let videos = video_scanner::scan_directory(folder.clone());
            
            eprintln!("[add_folder] Scanned {} videos from folder: {:?}", videos.len(), folder);
            eprintln!("[add_folder] Current database has {} videos", self.database.videos.len());
            
            // Get existing video paths for quick lookup (normalized for case-insensitive comparison)
            let existing_paths: HashSet<PathBuf> = self.database.videos.iter()
                .filter_map(|v| {
                    // Canonicalize path to handle case-insensitivity and path normalization
                    match v.path.canonicalize() {
                        Ok(p) => Some(p),
                        Err(e) => {
                            eprintln!("[add_folder] Warning: Failed to canonicalize existing path {:?}: {}", v.path, e);
                            None
                        }
                    }
                })
                .collect();
            
            eprintln!("[add_folder] Built index of {} existing canonical paths", existing_paths.len());
            
            // Filter out existing videos
            let new_videos: Vec<_> = videos.into_iter()
                .filter(|video| {
                    match video.path.canonicalize() {
                        Ok(canonical_path) => {
                            if existing_paths.contains(&canonical_path) {
                                eprintln!("[add_folder] Skipping existing video: {:?}", video.path);
                                false
                            } else {
                                true
                            }
                        }
                        Err(e) => {
                            eprintln!("[add_folder] Failed to canonicalize path {:?}: {}", video.path, e);
                            false
                        }
                    }
                })
                .collect();
            
            eprintln!("[add_folder] Found {} new videos to process", new_videos.len());
            
            // Apply free tier limit and process videos in parallel
            let current_count = self.database.videos.len();
            let processed_videos = if self.is_premium {
                // Premium: process all videos in parallel
                video_scanner::process_videos_parallel(new_videos, &cache_dir)
            } else {
                // Free tier: limit to 100 total videos
                let remaining_slots = 100_usize.saturating_sub(current_count);
                if remaining_slots == 0 {
                    eprintln!("[Free tier] Video limit reached. Cannot add more videos.");
                    self.show_premium_promotion_window = true;
                    Vec::new()
                } else {
                    if new_videos.len() > remaining_slots {
                        eprintln!("[Free tier] Limiting to {} videos (slots remaining)", remaining_slots);
                        self.show_premium_promotion_window = true;
                    }
                    video_scanner::process_videos_parallel_with_limit(new_videos, &cache_dir, remaining_slots)
                }
            };
            
            // Add processed videos to database
            for video in processed_videos {
                eprintln!("[add_folder] Adding new video: {:?}", video.path);
                self.database.add_video(video);
            }
            
            // Add folder to watched folders
            self.watched_folders.insert(folder.clone());
            self.setup_folder_watcher();
            
            // Save database
            let _ = database::save_database(&self.database);
        }
    }
    
    pub fn rescan_folders(&mut self) {
        // Only rescan folders that are in watched_folders (explicitly added by user)
        // Do not automatically scan all video parent folders
        let folders = self.watched_folders.clone();
        
        if folders.is_empty() {
            eprintln!("[rescan] No watched folders to scan");
            return;
        }
        
        eprintln!("[rescan] Rescanning {} watched folders", folders.len());
        
        let cache_dir = thumbnail::get_cache_dir();
        let mut videos_to_remove = Vec::new();
        
        // Check for deleted files
        for video in &self.database.videos {
            if !video.path.exists() {
                videos_to_remove.push(video.id.clone());
            }
        }
        
        // Remove deleted videos
        for video_id in videos_to_remove {
            eprintln!("[rescan] Removing deleted video: {}", video_id);
            self.delete_video(&video_id, true);
        }
        
        // Scan each folder for new or updated files
        for folder_path in folders {
            if !folder_path.exists() {
                eprintln!("[rescan] Skipping non-existent folder: {:?}", folder_path);
                continue;
            }
            
            eprintln!("[rescan] Scanning folder: {:?}", folder_path);
            
            // Build normalized path map for existing videos in this folder and subfolders
            let existing_paths: HashSet<PathBuf> = self.database.videos.iter()
                .filter(|v| {
                    // Check if video path starts with the folder path
                    v.path.starts_with(&folder_path)
                })
                .filter_map(|v| v.path.canonicalize().ok())
                .collect();
            
            eprintln!("[rescan] Found {} existing videos in this folder tree", existing_paths.len());
            
            let scanned_videos = video_scanner::scan_directory(folder_path.clone());
            
            // Separate new videos and existing videos that need updates
            let mut new_videos: Vec<_> = Vec::new();
            let mut videos_needing_update: Vec<PathBuf> = Vec::new();
            
            for video in scanned_videos {
                let canonical_path = match video.path.canonicalize() {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("[rescan] Failed to canonicalize path {:?}: {}", video.path, e);
                        continue;
                    }
                };
                
                if !existing_paths.contains(&canonical_path) {
                    eprintln!("[rescan] Found new video: {:?}", video.path);
                    new_videos.push(video);
                } else {
                    // Check if existing video needs metadata update
                    if let Some(existing_video) = self.database.get_video_by_path(&video.path) {
                        let needs_update = existing_video.duration.is_none() 
                            || existing_video.resolution.is_none()
                            || existing_video.thumbnail_path.is_none();
                        
                        // Check for file size change
                        let size_changed = if let Ok(metadata) = std::fs::metadata(&video.path) {
                            existing_video.file_size != metadata.len()
                        } else {
                            false
                        };
                        
                        if needs_update || size_changed {
                            videos_needing_update.push(video.path.clone());
                        }
                    }
                }
            }
            
            eprintln!("[rescan] Found {} new videos to process in parallel", new_videos.len());
            
            // Process new videos in parallel
            let processed_new_videos = video_scanner::process_videos_parallel(new_videos, &cache_dir);
            
            // Add new videos to database
            for video in processed_new_videos {
                self.database.add_video(video);
            }
            
            // Update existing videos that need metadata (this is typically fewer, so less critical)
            for video_path in videos_needing_update {
                if let Some(existing_video) = self.database.get_video_by_path_mut(&video_path) {
                    // Check if file was modified (by comparing file size)
                    if let Ok(metadata) = std::fs::metadata(&video_path) {
                        let file_size = metadata.len();
                        if existing_video.file_size != file_size {
                            eprintln!("[rescan] File size changed for: {:?}", video_path);
                            // Update file size
                            existing_video.file_size = file_size;
                            
                            // Regenerate thumbnail
                            existing_video.thumbnail_path = thumbnail::create_video_thumbnail(&video_path, &cache_dir);
                            
                            // Update metadata
                            existing_video.duration = get_video_duration(&video_path);
                            existing_video.resolution = get_video_resolution(&video_path);
                        }
                    }
                    
                    // Update missing metadata
                    if existing_video.duration.is_none() {
                        existing_video.duration = get_video_duration(&video_path);
                    }
                    if existing_video.resolution.is_none() {
                        existing_video.resolution = get_video_resolution(&video_path);
                    }
                    // Update missing thumbnail
                    if existing_video.thumbnail_path.is_none() {
                        existing_video.thumbnail_path = thumbnail::create_video_thumbnail(&video_path, &cache_dir);
                    }
                }
            }
        }
        
        // Update last rescan time
        self.last_rescan_time = SystemTime::now();
        self.pending_rescan = false;
        
        let _ = database::save_database(&self.database);
    }
    
    pub fn set_rating(&mut self, video_id: &str, rating: u8) {
        if let Some(video) = self.database.get_video_mut(video_id) {
            // Free tier: only favorite (1) or no favorite (0)
            // Premium: 1-5 star ratings
            if !self.is_premium && rating > 1 {
                video.rating = 1; // Cap at favorite for free tier
            } else {
                video.rating = rating.min(5); // Cap at 5 stars
            }
            let _ = database::save_database(&self.database);
        }
    }
    
    /// Activate a license key
    pub fn activate_license(&mut self, license_key: &str) {
        match license::verify_license(license_key) {
            Ok(license) if license.is_premium_active() => {
                self.current_license = Some(license.clone());
                self.is_premium = true;
                self.license_status_message = Some(format!(
                    "✅ License activated successfully!\nIssued to: {}\nExpires: {}",
                    license.info.issued_to,
                    license.info.expires_at.map(|ts| license::format_timestamp(ts))
                        .unwrap_or_else(|| "Never".to_string())
                ));
                
                // Save license key to settings (premium status derived from license)
                if let Ok(mut settings) = database::load_settings() {
                    settings.license_key = Some(license_key.to_string());
                    let _ = database::save_settings(&settings);
                }
                
                eprintln!("[License] Premium license activated for {}", license.info.issued_to);
            },
            Ok(license) if license.is_expired => {
                self.license_status_message = Some(format!(
                    "❌ License expired\nExpired on: {}",
                    license.info.expires_at.map(|ts| license::format_timestamp(ts))
                        .unwrap_or_else(|| "Unknown".to_string())
                ));
            },
            Ok(license) => {
                self.license_status_message = Some(format!(
                    "❌ Invalid license type: {}",
                    license.info.license_type
                ));
            },
            Err(e) => {
                self.license_status_message = Some(format!("❌ License verification failed:\n{}", e));
            }
        }
    }
    
    pub fn generate_scenes(&mut self, video_id: &str) {
        if let Some(video) = self.database.get_video_mut(video_id) {
            let cache_dir = thumbnail::get_cache_dir();
            let _ = scene_detection::detect_scenes(video, &cache_dir);
            let _ = database::save_database(&self.database);
        }
    }
    
    pub fn delete_scene(&mut self, video_id: &str, scene_timestamp: f64) {
        if let Some(video) = self.database.get_video_mut(video_id) {
            // Find and remove the scene
            if let Some(index) = video.scenes.iter().position(|s| (s.timestamp - scene_timestamp).abs() < 0.01) {
                let scene = video.scenes.remove(index);
                
                // Delete thumbnail file
                let _ = std::fs::remove_file(&scene.thumbnail_path);
                
                // Remove from texture cache
                self.texture_cache.remove(&scene.thumbnail_path);
                
                // Save database
                let _ = database::save_database(&self.database);
            }
        }
        
        // Clear scene selection
        self.selected_scenes.clear();
        self.last_selected_scene = None;
    }
    
    pub fn delete_selected_scenes(&mut self, video_id: &str) {
        if let Some(video) = self.database.get_video_mut(video_id) {
            // Sort indices in descending order to remove from back to front
            let mut indices: Vec<usize> = self.selected_scenes.iter().copied().collect();
            indices.sort_by(|a, b| b.cmp(a));
            
            for index in indices {
                if index < video.scenes.len() {
                    let scene = video.scenes.remove(index);
                    
                    // Delete thumbnail file
                    let _ = std::fs::remove_file(&scene.thumbnail_path);
                    
                    // Remove from texture cache
                    self.texture_cache.remove(&scene.thumbnail_path);
                }
            }
            
            // Save database
            let _ = database::save_database(&self.database);
        }
        
        // Clear scene selection
        self.selected_scenes.clear();
        self.last_selected_scene = None;
    }
    
    pub fn toggle_scene_selection(&mut self, index: usize) {
        if self.selected_scenes.contains(&index) {
            self.selected_scenes.remove(&index);
        } else {
            self.selected_scenes.insert(index);
        }
    }
    
    pub fn select_scene_range(&mut self, start: usize, end: usize) {
        let (min, max) = if start <= end { (start, end) } else { (end, start) };
        for i in min..=max {
            self.selected_scenes.insert(i);
        }
    }
    
    pub fn delete_video(&mut self, video_id: &str, delete_cache: bool) {
        // Get video info before deletion
        if let Some(video) = self.database.get_video(video_id) {
            // Delete cache files if requested
            if delete_cache {
                let cache_dir = thumbnail::get_cache_dir();
                
                // Delete main thumbnail
                if let Some(ref thumb_path) = video.thumbnail_path {
                    let _ = std::fs::remove_file(thumb_path);
                    // Remove from texture cache
                    self.texture_cache.remove(thumb_path);
                }
                
                // Delete scene thumbnails
                let scene_dir = cache_dir.join("scenes").join(&video.id);
                if scene_dir.exists() {
                    let _ = std::fs::remove_dir_all(&scene_dir);
                }
                
                // Remove scene textures from cache
                for scene in &video.scenes {
                    self.texture_cache.remove(&scene.thumbnail_path);
                }
            }
            
            // Remove from database
            self.database.remove_video(video_id);
            
            // Clear selection if deleted video was selected
            if self.selected_video.as_deref() == Some(video_id) {
                self.selected_video = None;
                self.scene_panel_visible = false;
            }
            
            // Remove from multi-selection if selected
            self.selected_videos.remove(video_id);
            
            // Save database
            let _ = database::save_database(&self.database);
        }
    }
    
    pub fn toggle_video_selection(&mut self, video_id: &str) {
        if self.selected_videos.contains(video_id) {
            self.selected_videos.remove(video_id);
        } else {
            self.selected_videos.insert(video_id.to_string());
        }
    }
    
    pub fn select_all_videos(&mut self, video_ids: Vec<String>) {
        self.selected_videos.clear();
        self.selected_videos.extend(video_ids);
    }
    
    pub fn clear_selection(&mut self) {
        self.selected_videos.clear();
        self.selected_video = None;
        self.last_selected_video = None;
    }
    
    pub fn select_range(&mut self, video_ids: &[String], from_id: &str, to_id: &str) {
        // Find indices of from and to
        let from_idx = video_ids.iter().position(|id| id == from_id);
        let to_idx = video_ids.iter().position(|id| id == to_id);
        
        if let (Some(from), Some(to)) = (from_idx, to_idx) {
            let start = from.min(to);
            let end = from.max(to);
            
            for id in &video_ids[start..=end] {
                self.selected_videos.insert(id.clone());
            }
        }
    }
    
    pub fn delete_selected_videos(&mut self, delete_cache: bool) {
        let video_ids: Vec<String> = self.selected_videos.iter().cloned().collect();
        for video_id in video_ids {
            self.delete_video(&video_id, delete_cache);
        }
        self.selected_videos.clear();
    }
    
    /// Setup file system watcher for monitored folders
    pub fn setup_folder_watcher(&mut self) {
        // Collect all folders to watch from database
        let folders: HashSet<PathBuf> = self.database.videos.iter()
            .filter_map(|v| v.path.parent().map(|p| p.to_path_buf()))
            .collect();
        
        self.watched_folders = folders.clone();
        
        // Create a channel for receiving events
        let (tx, rx) = channel();
        let rx = Arc::new(Mutex::new(rx));
        
        // Create a new watcher
        match notify::recommended_watcher(move |res| {
            if let Err(e) = tx.send(res) {
                eprintln!("Error sending watch event: {:?}", e);
            }
        }) {
            Ok(mut watcher) => {
                // Watch all folders
                for folder in &self.watched_folders {
                    if folder.exists() {
                        if let Err(e) = watcher.watch(folder, RecursiveMode::Recursive) {
                            eprintln!("Failed to watch folder {:?}: {:?}", folder, e);
                        } else {
                            eprintln!("[watcher] Watching folder: {:?}", folder);
                        }
                    }
                }
                
                self.fs_watcher = Some(watcher);
                self.fs_events = Some(rx);
            }
            Err(e) => {
                eprintln!("Failed to create file system watcher: {:?}", e);
            }
        }
    }
    
    /// Check for file system changes and trigger rescan if needed
    pub fn check_folder_changes(&mut self) {
        if let Some(rx) = &self.fs_events {
            if let Ok(rx_guard) = rx.lock() {
                // Check for pending events
                let mut has_changes = false;
                
                // Process all pending events
                while let Ok(result) = rx_guard.try_recv() {
                    match result {
                        Ok(event) => {
                            // Check if the event is related to video files
                            for path in &event.paths {
                                if path.is_file() && VideoFile::is_video_file(path) {
                                    eprintln!("[watcher] Detected change: {:?}", path);
                                    has_changes = true;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Watch error: {:?}", e);
                        }
                    }
                }
                
                // Set pending rescan flag if changes detected
                if has_changes {
                    // Only trigger rescan if enough time has passed since last rescan
                    if let Ok(elapsed) = self.last_rescan_time.elapsed() {
                        if elapsed.as_secs() >= 5 {
                            self.pending_rescan = true;
                        }
                    }
                }
            }
        }
    }
    
    /// Initialize the async texture loading channel if not already done
    fn ensure_texture_loader(&mut self) {
        if self.texture_load_sender.is_none() {
            let (tx, rx) = channel();
            self.texture_load_sender = Some(tx);
            self.texture_load_receiver = Some(rx);
        }
        if self.texture_fail_sender.is_none() {
            let (tx, rx) = channel();
            self.texture_fail_sender = Some(tx);
            self.texture_fail_receiver = Some(rx);
        }
    }
    
    /// Process any completed texture loads from background threads
    fn process_loaded_textures(&mut self, ctx: &egui::Context) {
        // Process successful loads
        if let Some(ref receiver) = self.texture_load_receiver {
            // Process up to 10 textures per frame to avoid blocking
            for _ in 0..10 {
                match receiver.try_recv() {
                    Ok((path, pixels, size)) => {
                        // Remove from pending set
                        self.pending_textures.remove(&path);
                        
                        // Create texture from received data
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                        let texture = ctx.load_texture(
                            path.to_string_lossy().to_string(),
                            color_image,
                            Default::default(),
                        );
                        self.texture_cache.insert(path, texture);
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // Channel disconnected, recreate it
                        self.texture_load_sender = None;
                        self.texture_load_receiver = None;
                        self.pending_textures.clear();
                        break;
                    }
                }
            }
        }
        
        // Process failed loads
        if let Some(ref receiver) = self.texture_fail_receiver {
            for _ in 0..10 {
                match receiver.try_recv() {
                    Ok(path) => {
                        // Remove from pending and add to failed
                        self.pending_textures.remove(&path);
                        self.failed_textures.insert(path);
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.texture_fail_sender = None;
                        self.texture_fail_receiver = None;
                        break;
                    }
                }
            }
        }
    }
    
    fn load_image_texture(&mut self, ctx: &egui::Context, image_path: &PathBuf) -> Option<egui::TextureHandle> {
        // Check if already cached - this is fast
        if let Some(texture) = self.texture_cache.get(image_path) {
            return Some(texture.clone());
        }
        
        // Check if already failed (don't retry failed images)
        if self.failed_textures.contains(image_path) {
            return None;
        }
        
        // Check if already loading
        if self.pending_textures.contains(image_path) {
            return None; // Still loading, return None to show placeholder
        }
        
        // Limit concurrent loading threads to prevent resource exhaustion
        const MAX_CONCURRENT_LOADS: usize = 8;
        if self.pending_textures.len() >= MAX_CONCURRENT_LOADS {
            return None; // Too many pending loads, wait for some to complete
        }
        
        // Ensure loader is initialized
        self.ensure_texture_loader();
        
        // Start async loading
        if let (Some(ref success_sender), Some(ref fail_sender)) = (&self.texture_load_sender, &self.texture_fail_sender) {
            let path = image_path.clone();
            let success_tx = success_sender.clone();
            let fail_tx = fail_sender.clone();
            let ctx_clone = ctx.clone();
            
            // Mark as pending
            self.pending_textures.insert(path.clone());
            
            // Spawn background thread to load image
            std::thread::spawn(move || {
                if let Some((pixels, size)) = load_image_data(&path) {
                    let _ = success_tx.send((path, pixels, size));
                } else {
                    // Send failure notification
                    let _ = fail_tx.send(path);
                }
                // Request repaint so the UI updates when texture is ready
                ctx_clone.request_repaint();
            });
        }
        
        None // Return None while loading, caller should show placeholder
    }
    
    fn draw_thumbnail_placeholder(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, thumbnail_size: egui::Vec2, video: &VideoFile) {
        let (rect, response) = ui.allocate_exact_size(thumbnail_size, egui::Sense::click());
        
        let is_selected = self.selected_videos.contains(&video.id) || 
                         self.selected_video.as_ref() == Some(&video.id);
        
        if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(rect, 4.0, egui::Color32::DARK_GRAY);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "🎬",
                egui::FontId::proportional(48.0 * self.thumbnail_scale),
                egui::Color32::WHITE,
            );
            
            // Draw selection highlight
            if is_selected {
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 200, 255))
                );
            }
        }
        
        if response.clicked() {
            let modifiers = ui.input(|i| i.modifiers.clone());
            if modifiers.shift {
                // Shift+Click: range selection (need video_ids context)
                self.selected_videos.insert(video.id.clone());
                self.last_selected_video = Some(video.id.clone());
                // Show profile of first selected video
                if self.selected_video.is_none() {
                    self.selected_video = Some(video.id.clone());
                    self.scene_panel_visible = true;
                }
            } else if modifiers.ctrl {
                self.toggle_video_selection(&video.id);
                self.last_selected_video = Some(video.id.clone());
                // Show profile of first selected video
                if self.selected_video.is_none() && self.selected_videos.contains(&video.id) {
                    self.selected_video = Some(video.id.clone());
                    self.scene_panel_visible = true;
                }
            } else {
                // Single click: clear multi-select and select only this video
                self.selected_videos.clear();
                self.selected_video = Some(video.id.clone());
                self.scene_panel_visible = true;
                self.last_selected_video = Some(video.id.clone());
            }
        }
        
        // Double click: play video
        if response.double_clicked() {
            let selected_shader = self.selected_shader.as_deref();
            let use_gpu_hq = self.is_premium && self.use_gpu_hq;
            let use_custom_shaders = self.is_premium && self.use_custom_shaders;
            let use_frame_interpolation = self.is_premium && self.use_frame_interpolation;
            if let Err(e) = video_player::play_video_at_timestamp(&video.path, 0.0, self.mpv_always_on_top, use_gpu_hq, use_custom_shaders, selected_shader, use_frame_interpolation, self.mpv_volume) {
                eprintln!("Video playback error: {}", e);
            }
        }
        
        response.context_menu(|ui| {
            if ui.button(&self.i18n.t("show_in_folder")).clicked() {
                if let Err(e) = video_player::show_in_folder(&video.path) {
                    eprintln!("Show in folder error: {}", e);
                }
                ui.close_menu();
            }
            
            ui.separator();
            
            if ui.button(&self.i18n.t("delete")).clicked() {
                self.delete_confirm_video = Some(video.id.clone());
                ui.close_menu();
            }
            
            // Multiple selection delete option
            if self.selected_videos.len() > 1 {
                ui.separator();
                
                let delete_multiple_text = self.i18n.t("delete_selected");
                if ui.button(&delete_multiple_text).clicked() {
                    self.delete_confirm_video = Some("__MULTI__".to_string());
                    ui.close_menu();
                }
            }
        });
    }
    
    pub fn add_tag_to_video(&mut self, video_id: &str, tag: String) {
        if let Some(video) = self.database.get_video_mut(video_id) {
            if !video.tags.contains(&tag) {
                video.tags.push(tag.clone());
                self.database.add_tag(tag);
                let _ = database::save_database(&self.database);
            }
        }
    }
    
    pub fn remove_tag_from_video(&mut self, video_id: &str, tag: &str) {
        if let Some(video) = self.database.get_video_mut(video_id) {
            video.tags.retain(|t| t != tag);
            let _ = database::save_database(&self.database);
            let _ = database::save_database(&self.database);
        }
    }
    
    pub fn set_video_folder(&mut self, video_id: &str, folder: String) {
        if let Some(video) = self.database.get_video_mut(video_id) {
            video.folder = Some(folder.clone());
            self.database.add_folder(folder);
        }
    }
    
    pub fn get_filtered_videos(&self) -> Vec<&VideoFile> {
        let mut videos: Vec<&VideoFile> = self.database.videos.iter().collect();
        
        // 評価フィルタ
        if self.min_rating_filter > 0 {
            videos.retain(|v| v.rating >= self.min_rating_filter);
        }
        
        // フォルダフィルタ（複数選択対応）
        if !self.selected_folder_filter.is_empty() {
            videos.retain(|v| {
                if let Some(folder) = &v.folder {
                    self.selected_folder_filter.contains(folder)
                } else {
                    false
                }
            });
        }
        
        // タグフィルタ（複数選択対応、AND/OR切り替え）
        if !self.selected_tag_filter.is_empty() {
            videos.retain(|v| {
                match self.tag_filter_mode {
                    FilterMode::Or => {
                        // OR: 選択されたタグのいずれかを持つ動画を表示
                        v.tags.iter().any(|t| self.selected_tag_filter.contains(t))
                    }
                    FilterMode::And => {
                        // AND: 選択されたタグをすべて持つ動画のみ表示
                        self.selected_tag_filter.iter().all(|selected_tag| v.tags.contains(selected_tag))
                    }
                }
            });
        }
        
        // 検索クエリ
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            videos.retain(|v| {
                v.title.to_lowercase().contains(&query)
                    || v.tags.iter().any(|t| t.to_lowercase().contains(&query))
            });
        }
        
        // ソート
        match self.sort_field {
            SortField::AddedDate => {
                videos.sort_by(|a, b| {
                    let cmp = a.added_date.cmp(&b.added_date);
                    if self.sort_order == SortOrder::Ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
            SortField::FileName => {
                videos.sort_by(|a, b| {
                    let cmp = a.title.to_lowercase().cmp(&b.title.to_lowercase());
                    if self.sort_order == SortOrder::Ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
            SortField::Duration => {
                videos.sort_by(|a, b| {
                    let cmp = a.duration.unwrap_or(0.0).partial_cmp(&b.duration.unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if self.sort_order == SortOrder::Ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
        }
        
        videos
    }
}

impl eframe::App for VideoPlayerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any completed texture loads from background threads
        self.process_loaded_textures(ctx);
        
        // Track window size and position for saving on exit
        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                self.last_window_size = Some((rect.width(), rect.height()));
            }
            if let Some(pos) = i.viewport().outer_rect {
                self.last_window_pos = Some((pos.min.x, pos.min.y));
            }
        });
        
        // Apply theme based on dark_mode setting
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
        
        // Check for file system changes
        self.check_folder_changes();
        
        // Perform rescan if pending
        if self.pending_rescan {
            eprintln!("[app] Triggering automatic rescan due to folder changes");
            self.rescan_folders();
        }
        
        // Initialize folder watcher if not set up yet and we have videos
        if self.fs_watcher.is_none() && !self.database.videos.is_empty() {
            self.setup_folder_watcher();
        }
        
        // トップバー
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // Title with background color
                let title_frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgb(60, 130, 60))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0));
                
                title_frame.show(ui, |ui| {
                    ui.heading(egui::RichText::new("CicadaGallery - Video Gallery Player").color(egui::Color32::WHITE));
                });
                
                ui.separator();
                
                if ui.button(&self.i18n.t("add_videos")).clicked() {
                    self.add_files();
                }
                
                if ui.button(&self.i18n.t("add_folder")).clicked() {
                    self.add_folder();
                }
                
                if ui.button(&self.i18n.t("rescan_folders")).clicked() {
                    self.rescan_folders();
                }
                
                ui.separator();
                
                let view_button_text = if self.current_view == ViewMode::Grid { 
                    self.i18n.t("list_view") 
                } else { 
                    self.i18n.t("grid_view") 
                };
                if ui.button(&view_button_text).clicked() {
                    self.current_view = if self.current_view == ViewMode::Grid {
                        ViewMode::List
                    } else {
                        ViewMode::Grid
                    };
                }
                
                ui.separator();
                
                if ui.button(&self.i18n.t("options")).clicked() {
                    self.show_options_window = !self.show_options_window;
                }
                
                ui.separator();
                
                // Scene panel button (premium only)
                if self.is_premium {
                    if ui.button(if self.scene_panel_visible { "Hide Scenes" } else { "Show Scenes" }).clicked() {
                        self.scene_panel_visible = !self.scene_panel_visible;
                    }
                } else {
                    ui.add_enabled(false, egui::Button::new("Show Scenes 🔒"));
                }
                
                ui.separator();
                
                // Multi-selection controls
                let filtered_videos = self.get_filtered_videos();
                if !filtered_videos.is_empty() {
                    if ui.button(&self.i18n.t("select_all")).clicked() {
                        let video_ids: Vec<String> = filtered_videos.iter().map(|v| v.id.clone()).collect();
                        self.select_all_videos(video_ids);
                    }
                    
                    if !self.selected_videos.is_empty() {
                        let clear_text = self.i18n.t("clear_selection_count").replace("{}", &self.selected_videos.len().to_string());
                        if ui.button(&clear_text).clicked() {
                            self.clear_selection();
                        }
                        
                        if ui.button(&self.i18n.t("add_tag_to_selected")).clicked() {
                            self.show_tag_add_popup = true;
                        }
                        
                        if ui.button(&self.i18n.t("delete_selected")).clicked() {
                            self.delete_confirm_video = Some("__MULTI__".to_string());
                        }
                    }
                }
                
                // Push volume slider to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::Slider::new(&mut self.mpv_volume, 0..=100)
                        .text("🔊")
                        .suffix("%")
                        .show_value(true)
                        .min_decimals(0)
                        .max_decimals(0));
                    // ui.label("デフォルト音量");
                    
                    ui.separator();
                    
                    // Show video count and limit for free tier
                    let video_count = self.database.videos.len();
                    if self.is_premium {
                        ui.label(format!("📹 Videos: {}", video_count));
                    } else {
                        let count_text = format!("📹 {}/100 (Free)", video_count);
                        let color = if video_count >= 100 {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else if video_count >= 80 {
                            egui::Color32::from_rgb(255, 200, 100)
                        } else {
                            egui::Color32::from_rgb(200, 200, 200)
                        };
                        ui.label(egui::RichText::new(count_text).color(color));
                    }
                    
                    if ui.memory(|mem| mem.is_anything_being_dragged()) {
                        self.save_settings();
                    }
                });
            });
        });
        
        // Sidebar (Filters)
        egui::SidePanel::left("side_panel").min_width(220.0).show(ctx, |ui| {
            ui.heading(&self.i18n.t("filters"));
            ui.separator();
            
            // Rating filter
            ui.label(&self.i18n.t("min_rating"));
            ui.horizontal_wrapped(|ui| {
                if ui.radio(self.min_rating_filter == 0, "All").clicked() {
                    self.min_rating_filter = 0;
                }
                
                if self.is_premium {
                    // Premium: 1-5 star ratings
                    for rating in 1..=5 {
                        let label = format!("{}★+", rating);
                        if ui.radio(self.min_rating_filter == rating, label).clicked() {
                            self.min_rating_filter = rating;
                        }
                    }
                } else {
                    // Free: Favorite only
                    if ui.radio(self.min_rating_filter == 1, "⭐ Favorites").clicked() {
                        self.min_rating_filter = 1;
                    }
                }
            });
            ui.separator();
            
            // Folder filter
            ui.label(&self.i18n.t("folders"));
            if ui.button(&self.i18n.t("all")).clicked() {
                self.selected_folder_filter.clear();
                self.last_selected_folder = None;
            }
            let folders_clone = self.database.folders.clone();
            egui::ScrollArea::vertical()
                .id_salt("folder_filter_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    for (idx, folder) in folders_clone.iter().enumerate() {
                        let is_selected = self.selected_folder_filter.contains(folder);
                        let response = ui.selectable_label(is_selected, folder);
                        
                        if response.clicked() {
                            let modifiers = ui.input(|i| i.modifiers);
                            
                            if self.is_premium {
                                // Premium: Ctrl+Click for multiple selection, Shift+Click for range selection
                                if modifiers.shift && self.last_selected_folder.is_some() {
                                    // Shift+Click: Range selection
                                    let last_idx = self.last_selected_folder.unwrap();
                                    let (start, end) = if idx < last_idx { (idx, last_idx) } else { (last_idx, idx) };
                                    for i in start..=end {
                                        if let Some(f) = folders_clone.get(i) {
                                            self.selected_folder_filter.insert(f.clone());
                                        }
                                    }
                                } else if modifiers.ctrl || modifiers.command {
                                    // Ctrl+Click: Toggle selection
                                    if is_selected {
                                        self.selected_folder_filter.remove(folder);
                                    } else {
                                        self.selected_folder_filter.insert(folder.clone());
                                    }
                                    self.last_selected_folder = Some(idx);
                                } else {
                                    // Normal click: Single selection (clear others)
                                    self.selected_folder_filter.clear();
                                    self.selected_folder_filter.insert(folder.clone());
                                    self.last_selected_folder = Some(idx);
                                }
                            } else {
                                // Free: Single selection only
                                self.selected_folder_filter.clear();
                                self.selected_folder_filter.insert(folder.clone());
                            }
                        }
                    }
                });
            
            ui.separator();
            
            // Tag filter
            ui.label(&self.i18n.t("tags_colon"));
            ui.horizontal(|ui| {
                if ui.button(&self.i18n.t("all")).clicked() {
                    self.selected_tag_filter.clear();
                    self.last_selected_tag = None;
                }
                // AND/OR toggle (only show when premium and multiple tags selected)
                if self.is_premium && self.selected_tag_filter.len() > 1 {
                    ui.separator();
                    if ui.selectable_label(self.tag_filter_mode == FilterMode::Or, "OR").clicked() {
                        self.tag_filter_mode = FilterMode::Or;
                    }
                    if ui.selectable_label(self.tag_filter_mode == FilterMode::And, "AND").clicked() {
                        self.tag_filter_mode = FilterMode::And;
                    }
                }
            });
            let tags_clone = self.database.tags.clone();
            egui::ScrollArea::vertical()
                .id_salt("tag_filter_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    for (idx, tag) in tags_clone.iter().enumerate() {
                        let is_selected = self.selected_tag_filter.contains(tag);
                        let response = ui.selectable_label(is_selected, tag);
                        
                        if response.clicked() {
                            let modifiers = ui.input(|i| i.modifiers);
                            
                            if self.is_premium {
                                // Premium: Ctrl+Click for multiple selection, Shift+Click for range selection
                                if modifiers.shift && self.last_selected_tag.is_some() {
                                    // Shift+Click: Range selection
                                    let last_idx = self.last_selected_tag.unwrap();
                                    let (start, end) = if idx < last_idx { (idx, last_idx) } else { (last_idx, idx) };
                                    for i in start..=end {
                                        if let Some(t) = tags_clone.get(i) {
                                            self.selected_tag_filter.insert(t.clone());
                                        }
                                    }
                                } else if modifiers.ctrl || modifiers.command {
                                    // Ctrl+Click: Toggle selection
                                    if is_selected {
                                        self.selected_tag_filter.remove(tag);
                                    } else {
                                        self.selected_tag_filter.insert(tag.clone());
                                    }
                                    self.last_selected_tag = Some(idx);
                                } else {
                                    // Normal click: Single selection (clear others)
                                    self.selected_tag_filter.clear();
                                    self.selected_tag_filter.insert(tag.clone());
                                    self.last_selected_tag = Some(idx);
                                }
                            } else {
                                // Free: Single selection only
                                self.selected_tag_filter.clear();
                                self.selected_tag_filter.insert(tag.clone());
                            }
                        }
                    }
                });
            
            ui.separator();
            
            // Statistics
            let total_text = self.i18n.t("total_videos").replace("{}", &self.database.videos.len().to_string());
            let rated_text = self.i18n.t("favorites_count").replace("{}", &self.database.get_by_rating(1).len().to_string());
            ui.label(&total_text);
            ui.label(&rated_text);
        });
        
        // Right side panel for scene thumbnails
        if self.scene_panel_visible {
            egui::SidePanel::right("scene_panel").exact_width(300.0).show(ctx, |ui| {
                if let Some(video_id) = &self.selected_video.clone() {
                    // Clone video data to avoid borrowing issues
                    let video_data = self.database.get_video(video_id).cloned();
                    
                    if let Some(video) = video_data {
                        // Check if metadata is complete (both duration and resolution present)
                        let metadata_complete = video.duration.is_some() && video.resolution.is_some();
                        
                        // Only load metadata if incomplete and not yet marked as loaded
                        // Skip the actual loading to prevent UI blocking
                        if !metadata_complete && !self.metadata_loaded.contains(video_id) {
                            // Mark as loaded to prevent repeated checks
                            self.metadata_loaded.insert(video_id.clone());
                        } else if metadata_complete && !self.metadata_loaded.contains(video_id) {
                            // If metadata is already complete, just mark as loaded
                            self.metadata_loaded.insert(video_id.clone());
                        }
                        
                        // Display selected video information
                        ui.heading(&self.i18n.t("selected_video"));
                        ui.separator();
                        
                        // Display video thumbnail
                        if let Some(ref thumb_path) = video.thumbnail_path {
                            if let Some(texture) = self.load_image_texture(ctx, thumb_path) {
                                let thumbnail_size = egui::vec2(280.0, 157.0);
                                ui.add(
                                    egui::Image::new(&texture)
                                        .max_size(thumbnail_size)
                                );
                            }
                        } else {
                            // Placeholder if no thumbnail
                            let thumbnail_size = egui::vec2(280.0, 157.0);
                            let (rect, _) = ui.allocate_exact_size(thumbnail_size, egui::Sense::hover());
                            if ui.is_rect_visible(rect) {
                                ui.painter().rect_filled(rect, 4.0, egui::Color32::DARK_GRAY);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &self.i18n.t("no_thumbnail"),
                                    egui::FontId::proportional(16.0),
                                    egui::Color32::WHITE,
                                );
                            }
                        }
                        
                        // Display video title
                        ui.add_space(5.0);
                        ui.label(egui::RichText::new(&video.title).strong().size(14.0));
                        
                        ui.add_space(5.0);
                        
                        // Display video information
                        ui.group(|ui| {
                            ui.set_width(ui.available_width());
                            
                            // Duration
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⏱").size(14.0));
                                if let Some(duration) = video.duration {
                                    let duration_text = self.i18n.t("duration_label").replace("{}", &scene_detection::format_timestamp(duration));
                                    ui.label(&duration_text);
                                } else {
                                    ui.label(self.i18n.t("duration_label").replace("{}", "-"));
                                }
                            });
                            
                            // Resolution
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("📐").size(14.0));
                                if let Some((width, height)) = video.resolution {
                                    let resolution_text = self.i18n.t("resolution_label").replace("{}", &format!("{}×{}", width, height));
                                    ui.label(&resolution_text);
                                } else {
                                    ui.label(self.i18n.t("resolution_label").replace("{}", "-"));
                                }
                            });
                            
                            // File size
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("💾").size(14.0));
                                let size_mb = video.file_size as f64 / 1024.0 / 1024.0;
                                let size_text = if size_mb >= 1024.0 {
                                    self.i18n.t("size_gb").replace("{:.2}", &format!("{:.2}", size_mb / 1024.0))
                                } else {
                                    self.i18n.t("size_mb").replace("{:.1}", &format!("{:.1}", size_mb))
                                };
                                ui.label(&size_text);
                            });
                            
                            // Folder
                            if let Some(ref folder) = video.folder {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("📁").size(14.0));
                                    let folder_text = self.i18n.t("folder_label").replace("{}", folder);
                                    ui.label(&folder_text);
                                });
                            }
                            
                            // Tags
                            if !video.tags.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("🏷").size(14.0));
                                    let tags_text = self.i18n.t("tags_label").replace("{}", &video.tags.join(", "));
                                    ui.label(&tags_text);
                                });
                            }
                            
                            // Added date
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("📅").size(14.0));
                                let added_text = self.i18n.t("added_label").replace("{}", &video.added_date.format("%Y-%m-%d %H:%M").to_string());
                                ui.label(&added_text);
                            });
                            
                            // Last played
                            if let Some(last_played) = video.last_played {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("▶").size(14.0));
                                    let last_played_text = self.i18n.t("last_played_label").replace("{}", &last_played.format("%Y-%m-%d %H:%M").to_string());
                                    ui.label(&last_played_text);
                                });
                            }
                        });
                        
                        ui.add_space(5.0);
                        
                        // Star rating
                        ui.horizontal(|ui| {
                            ui.label(&self.i18n.t("rating"));
                            let current_rating = video.rating;
                            for star in 1..=5 {
                                let star_text = if star <= current_rating {
                                    "★"
                                } else {
                                    "☆"
                                };
                                if ui.button(egui::RichText::new(star_text).size(24.0)).clicked() {
                                    // Toggle: if clicking current rating, set to 0 (no rating)
                                    let new_rating = if current_rating == star { 0 } else { star };
                                    self.set_rating(video_id, new_rating);
                                }
                            }
                            if current_rating > 0 && ui.small_button("×").clicked() {
                                self.set_rating(video_id, 0);
                            }
                        });
                        
                        ui.add_space(10.0);
                        ui.separator();
                        
                        // Tag editing section
                        ui.heading(&self.i18n.t("tags"));
                        
                        let video_tags = video.tags.clone();
                        let video_id_for_tags = video_id.clone();
                        
                        ui.horizontal_wrapped(|ui| {
                            ui.label(&self.i18n.t("tags_colon"));
                            
                            // Display existing tags with remove button
                            for tag in &video_tags {
                                ui.horizontal(|ui| {
                                    let _tag_label = ui.label(format!("#{}", tag));
                                    if ui.small_button("×").clicked() {
                                        self.remove_tag_from_video(&video_id_for_tags, tag);
                                    }
                                });
                            }
                            
                            // Add tag button
                            if ui.small_button("+").clicked() {
                                self.show_tag_add_popup = true;
                            }
                        });
                        
                        // Tag addition popup
                        if self.show_tag_add_popup {
                            let screen_rect = ui.ctx().screen_rect();
                            let popup_pos = egui::pos2(
                                screen_rect.max.x - 350.0,  // 右から350px
                                screen_rect.center().y - 150.0  // 中央より少し上
                            );
                            
                            // Determine target videos (single or multiple selection)
                            let target_videos: Vec<String> = if self.selected_videos.is_empty() {
                                vec![video_id_for_tags.clone()]
                            } else {
                                // 複数選択時はselected_videoも含める
                                let mut vids: Vec<String> = self.selected_videos.iter().cloned().collect();
                                if let Some(sel) = &self.selected_video {
                                    if !vids.contains(sel) {
                                        vids.push(sel.clone());
                                    }
                                }
                                vids
                            };
                            
                            let is_multi = target_videos.len() > 1;
                            let title = if is_multi {
                                format!("{} ({} videos)", self.i18n.t("add_tag"), target_videos.len())
                            } else {
                                self.i18n.t("add_tag")
                            };
                            
                            egui::Window::new(&title)
                                .collapsible(false)
                                .resizable(false)
                                .default_pos(popup_pos)
                                .show(ui.ctx(), |ui| {
                                    ui.label(&self.i18n.t("select_or_create_tag"));
                                    
                                    ui.separator();
                                    
                                    // Show existing tags in database
                                    ui.label(&self.i18n.t("existing_tags"));
                                    egui::ScrollArea::vertical()
                                        .max_height(150.0)
                                        .show(ui, |ui| {
                                            let all_tags = self.database.tags.clone();
                                            for tag in &all_tags {
                                                // For multi-selection, show all tags
                                                // For single selection, don't show tags already assigned
                                                let should_show = if is_multi {
                                                    true
                                                } else {
                                                    !video_tags.contains(tag)
                                                };
                                                
                                                if should_show {
                                                    if ui.button(format!("#{}", tag)).clicked() {
                                                        // Add tag to all target videos
                                                        for vid in &target_videos {
                                                            self.add_tag_to_video(vid, tag.clone());
                                                        }
                                                        self.show_tag_add_popup = false;
                                                    }
                                                }
                                            }
                                        });
                                    
                                    ui.separator();
                                    
                                    // Create new tag
                                    ui.label(&self.i18n.t("create_new_tag"));
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut self.new_tag_input);
                                        if ui.button(&self.i18n.t("create_tag")).clicked() && !self.new_tag_input.is_empty() {
                                            let new_tag = self.new_tag_input.trim().to_string();
                                            if !new_tag.is_empty() {
                                                // Add tag to all target videos
                                                for vid in &target_videos {
                                                    self.add_tag_to_video(vid, new_tag.clone());
                                                }
                                                self.new_tag_input.clear();
                                                self.show_tag_add_popup = false;
                                            }
                                        }
                                    });
                                    
                                    ui.separator();
                                    
                                    if ui.button(&self.i18n.t("cancel")).clicked() {
                                        self.show_tag_add_popup = false;
                                        self.new_tag_input.clear();
                                    }
                                });
                        }
                        
                        ui.add_space(10.0);
                        ui.separator();
                        
                        // Scene thumbnails section (premium only)
                        if self.is_premium {
                            ui.heading(&self.i18n.t("scene_thumbnails"));
                            
                            // Show selection controls if scenes are selected
                            if !self.selected_scenes.is_empty() {
                                ui.horizontal(|ui| {
                                    let selected_text = self.i18n.t("selected_count").replace("{}", &self.selected_scenes.len().to_string());
                                    ui.label(&selected_text);
                                    if ui.button(&self.i18n.t("clear_selection")).clicked() {
                                        self.selected_scenes.clear();
                                        self.last_selected_scene = None;
                                    }
                                    if ui.button(&self.i18n.t("delete_selected")).clicked() {
                                        self.delete_selected_scenes(video_id);
                                    }
                                });
                            }
                            
                            ui.separator();
                        // Show generate button if no scenes exist
                        if video.scenes.is_empty() {
                            ui.label(&self.i18n.t("no_scenes_yet"));
                            if ui.button(&self.i18n.t("generate_scenes")).clicked() {
                                self.generate_scenes(video_id);
                            }
                        } else {
                            // Display scene thumbnails in a scrollable area
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let scenes = video.scenes.clone();
                                    let video_path = video.path.clone();
                                    let video_id_clone = video_id.clone();
                                    
                                    for (scene_index, scene) in scenes.iter().enumerate() {
                                        let is_selected = self.selected_scenes.contains(&scene_index);
                                        
                                        ui.group(|ui| {
                                            let thumbnail_size = egui::vec2(180.0, 101.0);
                                            let (rect, response) = ui.allocate_exact_size(thumbnail_size, egui::Sense::click());
                                            
                                            // Only load texture if visible (optimization)
                                            if ui.is_rect_visible(rect) {
                                                // Load and display actual thumbnail image
                                                if let Some(texture) = self.load_image_texture(ctx, &scene.thumbnail_path) {
                                                    // Draw thumbnail
                                                    ui.put(rect, egui::Image::new(&texture).fit_to_exact_size(thumbnail_size));
                                                    
                                                    // Draw selection highlight
                                                    if is_selected {
                                                        ui.painter().rect_stroke(
                                                            rect,
                                                            4.0,
                                                            egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 200, 255))
                                                        );
                                                    }
                                                }
                                            }
                                            
                                            // Handle click with modifiers (always handle regardless of visibility)
                                            if response.clicked() {
                                                let modifiers = ui.input(|i| i.modifiers.clone());
                                                if modifiers.shift {
                                                    // Shift+Click: range selection
                                                    if let Some(last_idx) = self.last_selected_scene {
                                                        self.select_scene_range(last_idx, scene_index);
                                                    } else {
                                                        self.selected_scenes.insert(scene_index);
                                                    }
                                                    self.last_selected_scene = Some(scene_index);
                                                } else if modifiers.ctrl {
                                                    // Ctrl+Click: toggle selection
                                                    self.toggle_scene_selection(scene_index);
                                                    self.last_selected_scene = Some(scene_index);
                                                } else {
                                                    // Normal click: play video
                                                    self.selected_scenes.clear();
                                                    self.last_selected_scene = None;
                                                    let selected_shader = self.selected_shader.as_deref();
                                                    let use_gpu_hq = self.is_premium && self.use_gpu_hq;
                                                    let use_custom_shaders = self.is_premium && self.use_custom_shaders;
                                                    let use_frame_interpolation = self.is_premium && self.use_frame_interpolation;
                                                    if let Err(e) = video_player::play_video_at_timestamp(&video_path, scene.timestamp, self.mpv_always_on_top, use_gpu_hq, use_custom_shaders, selected_shader, use_frame_interpolation, self.mpv_volume) {
                                                        eprintln!("Video playback error: {}", e);
                                                    }
                                                }
                                            }
                                            
                                            // Right-click menu for scene operations
                                            let scene_ts = scene.timestamp;
                                            response.context_menu(|ui| {
                                                if ui.button(&self.i18n.t("play_from_scene")).clicked() {
                                                    let selected_shader = self.selected_shader.as_deref();
                                                    let use_gpu_hq = self.is_premium && self.use_gpu_hq;
                                                    let use_custom_shaders = self.is_premium && self.use_custom_shaders;
                                                    let use_frame_interpolation = self.is_premium && self.use_frame_interpolation;
                                                    if let Err(e) = video_player::play_video_at_timestamp(&video_path, scene_ts, self.mpv_always_on_top, use_gpu_hq, use_custom_shaders, selected_shader, use_frame_interpolation, self.mpv_volume) {
                                                        eprintln!("Video playback error: {}", e);
                                                    }
                                                    ui.close_menu();
                                                }
                                                
                                                ui.separator();
                                                
                                                if ui.button(&self.i18n.t("delete_scene")).clicked() {
                                                    self.delete_scene(&video_id_clone, scene_ts);
                                                    ui.close_menu();
                                                }
                                            });
                                            
                                            response.on_hover_text(&self.i18n.t("click_play_ctrl_select"));
                                            
                                            // Timestamp label
                                            ui.label(scene_detection::format_timestamp(scene.timestamp));
                                        });
                                        
                                        ui.add_space(5.0);
                                    }
                                });
                        }
                        } else {
                            // Free tier: Show locked message for scene thumbnails
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.heading(&self.i18n.t("scene_thumbnails_locked"));
                                ui.add_space(10.0);
                                ui.label(&self.i18n.t("premium_feature_available"));
                                ui.add_space(5.0);
                                ui.label(&self.i18n.t("premium_features"));
                                ui.label(&self.i18n.t("premium_scene_generation"));
                                ui.label(&self.i18n.t("premium_star_ratings"));
                                ui.label(&self.i18n.t("premium_glsl_shaders"));
                                ui.label(&self.i18n.t("premium_frame_interpolation"));
                                ui.label(&self.i18n.t("premium_gpu_rendering"));
                                ui.label(&self.i18n.t("premium_unlimited_storage"));
                                ui.label(&self.i18n.t("premium_multi_select"));
                            });
                        }
                    }
                } else {
                    // No video selected
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading(&self.i18n.t("selected_video"));
                        ui.add_space(10.0);
                        ui.label(&self.i18n.t("no_video_selected"));
                    });
                }
            });
        }
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Search bar with sort buttons
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
                
                ui.separator();
                ui.label(self.i18n.t("sort"));
                
                // 作成日時ボタン
                let added_date_text = match self.sort_field {
                    SortField::AddedDate => {
                        if self.sort_order == SortOrder::Ascending {
                            self.i18n.t("sort_added_date_asc")
                        } else {
                            self.i18n.t("sort_added_date_desc")
                        }
                    }
                    _ => self.i18n.t("sort_added_date")
                };
                if ui.button(added_date_text).clicked() {
                    if self.sort_field == SortField::AddedDate {
                        // 同じフィールドの場合は昇順降順を切り替え
                        self.sort_order = if self.sort_order == SortOrder::Ascending {
                            SortOrder::Descending
                        } else {
                            SortOrder::Ascending
                        };
                    } else {
                        // 違うフィールドの場合は昇順で開始
                        self.sort_field = SortField::AddedDate;
                        self.sort_order = SortOrder::Ascending;
                    }
                }
                
                // ファイル名ボタン
                let file_name_text = match self.sort_field {
                    SortField::FileName => {
                        if self.sort_order == SortOrder::Ascending {
                            self.i18n.t("sort_filename_asc")
                        } else {
                            self.i18n.t("sort_filename_desc")
                        }
                    }
                    _ => self.i18n.t("sort_filename")
                };
                if ui.button(file_name_text).clicked() {
                    if self.sort_field == SortField::FileName {
                        self.sort_order = if self.sort_order == SortOrder::Ascending {
                            SortOrder::Descending
                        } else {
                            SortOrder::Ascending
                        };
                    } else {
                        self.sort_field = SortField::FileName;
                        self.sort_order = SortOrder::Ascending;
                    }
                }
                
                // 動画時間ボタン
                let duration_text = match self.sort_field {
                    SortField::Duration => {
                        if self.sort_order == SortOrder::Ascending {
                            self.i18n.t("sort_duration_asc")
                        } else {
                            self.i18n.t("sort_duration_desc")
                        }
                    }
                    _ => self.i18n.t("sort_duration")
                };
                if ui.button(duration_text).clicked() {
                    if self.sort_field == SortField::Duration {
                        self.sort_order = if self.sort_order == SortOrder::Ascending {
                            SortOrder::Descending
                        } else {
                            SortOrder::Ascending
                        };
                    } else {
                        self.sort_field = SortField::Duration;
                        self.sort_order = SortOrder::Ascending;
                    }
                }
            });
            
            ui.separator();
            
            // Display video list
            let current_view = self.current_view.clone();
            let videos_to_show: Vec<VideoFile> = self.get_filtered_videos()
                .into_iter()
                .cloned()
                .collect();
            
            // Remember the selection state BEFORE drawing UI
            let selection_before = self.selected_video.clone();
            let selections_before = self.selected_videos.clone();
            
            // Create a scrollable area for content
            egui::ScrollArea::vertical().show(ui, |ui| {
                match current_view {
                    ViewMode::Grid => self.show_grid_view(ui, ctx, &videos_to_show),
                    ViewMode::List => self.show_list_view(ui, &videos_to_show),
                }
            });
            
            // Check for background click: if mouse was clicked but selection didn't change,
            // it means user clicked on empty area
            let left_clicked = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));
            let selection_changed = selection_before != self.selected_video || 
                                   selections_before != self.selected_videos;
            
            // Check if any popup, window, or context menu is open
            let any_popup_open = self.show_options_window || 
                                self.show_folder_management_window || 
                                self.show_tag_management_window ||
                                self.show_shader_management_window ||
                                self.show_license_window ||
                                self.show_premium_promotion_window ||
                                self.show_tag_add_popup ||
                                self.delete_confirm_video.is_some() ||
                                self.folder_delete_confirm.is_some() ||
                                self.tag_delete_confirm.is_some();
            
            // Also check if something is being interacted with (context menu, etc.)
            let is_using_pointer = ctx.is_using_pointer();
            
            // If clicked in this panel area and selection didn't change, clear selection
            // But skip if any popup/window is open or context menu is active
            if left_clicked && !selection_changed && !any_popup_open && !is_using_pointer {
                // User clicked somewhere but not on a thumbnail
                // Check if click was in our panel area
                let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
                if let Some(pos) = pointer_pos {
                    let panel_rect = ui.min_rect();
                    if panel_rect.contains(pos) {
                        // Clicked in our panel but not on any thumbnail
                        if self.selected_video.is_some() || !self.selected_videos.is_empty() {
                            self.clear_selection();
                        }
                    }
                }
            }
        });
        
        // Options window
        let options_window_was_open = self.show_options_window;
        let mut settings_changed = false;
        
        if self.show_options_window {
            egui::Window::new(&self.i18n.t("options"))
                .open(&mut self.show_options_window)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.heading(&self.i18n.t("display_settings"));
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label(&self.i18n.t("thumbnail_scale"));
                        if ui.add(egui::Slider::new(&mut self.thumbnail_scale, 0.5..=3.0)
                            .text("Scale")
                            .suffix("x")).changed() {
                            settings_changed = true;
                        }
                    });
                    
                    ui.label(format!("Current scale: {:.0}%", self.thumbnail_scale * 100.0));
                    
                    ui.separator();
                    
                    if ui.checkbox(&mut self.show_full_filename, &self.i18n.t("show_full_filename")).changed() {
                        settings_changed = true;
                    }
                    if ui.checkbox(&mut self.show_tags_in_grid, &self.i18n.t("show_tags_in_grid")).changed() {
                        settings_changed = true;
                    }
                    
                    ui.separator();
                    ui.heading(&self.i18n.t("theme"));
                    ui.separator();
                    
                    if ui.checkbox(&mut self.dark_mode, &self.i18n.t("dark_mode")).changed() {
                        settings_changed = true;
                    }
                    
                    ui.separator();
                    
                    ui.label(&self.i18n.t("language"));
                    let current_language = self.i18n.get_language();
                    for lang in Language::all() {
                        if ui.radio(current_language == lang, lang.name()).clicked() {
                            self.i18n.set_language(lang);
                            settings_changed = true;
                        }
                    }
                    
                    ui.separator();
                    ui.heading(&self.i18n.t("player_settings"));
                    ui.separator();
                    
                    if ui.checkbox(&mut self.mpv_always_on_top, &self.i18n.t("always_on_top")).changed() {
                        settings_changed = true;
                    }
                    
                    // GPU settings - only show for premium users
                    if self.is_premium {
                        ui.add_space(5.0);
                        
                        if ui.checkbox(&mut self.use_gpu_hq, &self.i18n.t("use_gpu_hq")).changed() {
                            settings_changed = true;
                        }
                        ui.label("  GPU high-quality mode uses advanced shaders");
                        
                        if ui.checkbox(&mut self.use_frame_interpolation, &self.i18n.t("use_frame_interpolation")).changed() {
                            settings_changed = true;
                        }
                        ui.label("  Frame interpolation enables smooth motion");
                        
                        ui.separator();
                        
                        if ui.checkbox(&mut self.use_custom_shaders, &self.i18n.t("use_custom_shaders")).changed() {
                            settings_changed = true;
                        }
                        
                        // Button to open shader management window
                        if self.use_custom_shaders {
                            if ui.button(&self.i18n.t("manage_shaders")).clicked() {
                                self.show_shader_management_window = true;
                            }
                        } else {
                            ui.label("  Place .glsl shader files in mpv/glsl_shaders");
                        }
                    }
                    
                    ui.separator();
                    ui.heading(&self.i18n.t("management"));
                    ui.separator();
                    
                    // Button to open folder management window
                    if ui.button(&self.i18n.t("manage_folders")).clicked() {
                        self.show_folder_management_window = true;
                    }
                    
                    // Button to open tag management window
                    if ui.button(&self.i18n.t("manage_tags")).clicked() {
                        self.show_tag_management_window = true;
                    }
                    
                    // Button to open/show MPV shortcuts panel
                    if ui.button("Show MPV Shortcuts").clicked() {
                        self.mpv_shortcuts_open = true;
                        settings_changed = true;
                    }
                    
                    ui.separator();
                    
                    if ui.button("Reset to Default").clicked() {
                        self.thumbnail_scale = 1.0;
                        self.mpv_always_on_top = true;
                        self.show_full_filename = false;
                        self.show_tags_in_grid = true;
                        self.dark_mode = false;
                        self.use_gpu_hq = false;
                        self.use_custom_shaders = false;
                        self.selected_shader = None;
                        self.use_frame_interpolation = false;
                        settings_changed = true;
                    }
                    
                    // License section
                    ui.separator();
                    ui.heading("Premium License");
                    ui.separator();
                    
                    if let Some(ref license) = self.current_license {
                        // Show license info
                        ui.horizontal(|ui| {
                            ui.label(&self.i18n.t("issued_to"));
                            ui.label(&license.info.issued_to);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(&self.i18n.t("expires"));
                            let expires_text = license.info.expires_at
                                .map(|ts| license::format_timestamp(ts))
                                .unwrap_or_else(|| self.i18n.t("never_expires"));
                            ui.label(&expires_text);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(&self.i18n.t("license_status"));
                            if license.is_expired {
                                ui.label(egui::RichText::new("❌ Expired").color(egui::Color32::RED));
                            } else if license.is_valid {
                                ui.label(egui::RichText::new("✅ Active").color(egui::Color32::GREEN));
                            } else {
                                ui.label(egui::RichText::new("❌ Invalid").color(egui::Color32::RED));
                            }
                        });
                        
                        ui.add_space(5.0);
                        if ui.button(&self.i18n.t("enter_license_key")).clicked() {
                            self.show_license_window = true;
                        }
                    } else {
                        // Premium promotion for free users
                        ui.label(egui::RichText::new(&self.i18n.t("premium_benefits_title")).strong());
                        ui.add_space(5.0);
                        
                        ui.label(&self.i18n.t("premium_benefit_1")); // 5つ星レーティング
                        ui.label(&self.i18n.t("premium_benefit_2")); // 複数タグ/フォルダ選択
                        ui.label(&self.i18n.t("premium_benefit_3")); // GPU高画質レンダリング
                        ui.label(&self.i18n.t("premium_benefit_4")); // カスタムシェーダー
                        ui.label(&self.i18n.t("premium_benefit_5")); // フレーム補間
                        ui.label(&self.i18n.t("premium_benefit_6")); // 無制限の動画プロファイル
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button(&self.i18n.t("purchase_premium")).clicked() {
                                // Open purchase page using platform-specific command
                                #[cfg(target_os = "windows")]
                                {
                                    let _ = std::process::Command::new("cmd")
                                        .args(["/C", "start", "", "https://cicadagallery.com/premium"])
                                        .spawn();
                                }
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = std::process::Command::new("open")
                                        .arg("https://cicadagallery.com/premium")
                                        .spawn();
                                }
                                #[cfg(target_os = "linux")]
                                {
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg("https://cicadagallery.com/premium")
                                        .spawn();
                                }
                            }
                            
                            if ui.button(&self.i18n.t("enter_license_key")).clicked() {
                                self.show_license_window = true;
                            }
                        });
                    }
                });
        }
        
        // Save settings when changed or when options window is closed
        if settings_changed || (options_window_was_open && !self.show_options_window) {
            self.save_settings();
        }
        
        // Folder Management Window
        if self.show_folder_management_window {
            let folder_management_title = self.i18n.t("folder_management");
            let registered_folders_text = self.i18n.t("registered_folders");
            let new_folder_name_text = self.i18n.t("new_folder_name");
            let add_folder_name_text = self.i18n.t("add_folder_name");
            
            let mut window_open = self.show_folder_management_window;
            
            egui::Window::new(&folder_management_title)
                .open(&mut window_open)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.label(&registered_folders_text);
                    ui.separator();
                    
                    let mut folder_to_confirm: Option<(String, usize)> = None;
                    let mut folders_changed = false;
                    
                    // Display folders with delete buttons
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            let folders = self.database.folders.clone();
                            for folder in folders {
                                ui.horizontal(|ui| {
                                    ui.label(&folder);
                                    if ui.button("❌").clicked() {
                                        // Count videos in this folder
                                        let video_count = self.database.videos.iter()
                                            .filter(|v| v.folder.as_ref() == Some(&folder))
                                            .count();
                                        folder_to_confirm = Some((folder.clone(), video_count));
                                    }
                                });
                            }
                        });
                    
                    // Set confirmation if requested
                    if let Some((folder, count)) = folder_to_confirm {
                        self.folder_delete_confirm = Some((folder, count));
                    }
                    
                    ui.separator();
                    
                    // Add new folder
                    ui.label(&new_folder_name_text);
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_folder_input);
                        if ui.button(&add_folder_name_text).clicked() && !self.new_folder_input.trim().is_empty() {
                            let new_folder = self.new_folder_input.trim().to_string();
                            self.database.add_folder(new_folder);
                            self.new_folder_input.clear();
                            folders_changed = true;
                        }
                    });
                    
                    if folders_changed {
                        self.save_settings();
                        let _ = database::save_database(&self.database);
                    }
                });
            
            self.show_folder_management_window = window_open;
        }
        
        // Tag Management Window
        if self.show_tag_management_window {
            let tag_management_title = self.i18n.t("tag_management");
            let registered_tags_text = self.i18n.t("registered_tags");
            
            let mut window_open = self.show_tag_management_window;
            
            egui::Window::new(&tag_management_title)
                .open(&mut window_open)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label(&registered_tags_text);
                    ui.separator();
                    
                    let mut tag_to_confirm: Option<(String, usize)> = None;
                    
                    // Display tags with delete buttons
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            let tags = self.database.tags.clone();
                            for tag in tags {
                                ui.horizontal(|ui| {
                                    ui.label(format!("🏷 {}", &tag));
                                    if ui.button("❌").clicked() {
                                        // Count videos using this tag
                                        let video_count = self.database.videos.iter()
                                            .filter(|v| v.tags.contains(&tag))
                                            .count();
                                        tag_to_confirm = Some((tag.clone(), video_count));
                                    }
                                });
                            }
                            
                            if self.database.tags.is_empty() {
                                ui.label("(No tags registered)");
                            }
                        });
                    
                    // Set confirmation if requested
                    if let Some((tag, count)) = tag_to_confirm {
                        self.tag_delete_confirm = Some((tag, count));
                    }
                });
            
            self.show_tag_management_window = window_open;
        }
        
        // Tag Delete Confirmation Window
        if self.tag_delete_confirm.is_some() {
            let confirm_title = self.i18n.t("confirm_tag_delete_title");
            let confirm_msg = self.i18n.t("confirm_tag_delete");
            let cancel_text = self.i18n.t("cancel");
            
            let mut should_close = false;
            let mut delete_tag = false;
            
            egui::Window::new(&confirm_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if let Some((tag_name, video_count)) = &self.tag_delete_confirm {
                        ui.label(format!("🏷 {}", tag_name));
                        ui.separator();
                        ui.label(&confirm_msg);
                        
                        if *video_count > 0 {
                            ui.add_space(8.0);
                            let videos_msg = self.i18n.t("tag_used_in_videos")
                                .replace("{}", &video_count.to_string());
                            ui.label(egui::RichText::new(videos_msg).color(egui::Color32::YELLOW));
                            ui.label(self.i18n.t("tag_will_be_removed"));
                        }
                        
                        ui.add_space(16.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("OK").color(egui::Color32::RED)).clicked() {
                                delete_tag = true;
                                should_close = true;
                            }
                            if ui.button(&cancel_text).clicked() {
                                should_close = true;
                            }
                        });
                    }
                });
            
            // Process deletion after window is closed
            if should_close {
                if let Some((tag, _video_count)) = self.tag_delete_confirm.take() {
                    if delete_tag {
                        // Remove tag from tags list
                        self.database.tags.retain(|t| t != &tag);
                        
                        // Remove tag from all videos
                        for video in &mut self.database.videos {
                            video.tags.retain(|t| t != &tag);
                        }
                        
                        // Also remove from selected tag filter
                        self.selected_tag_filter.remove(&tag);
                        
                        eprintln!("[tag_management] Removed tag '{}' from database and all videos", tag);
                        
                        self.save_settings();
                        let _ = database::save_database(&self.database);
                    }
                }
            }
        }
        
        // Folder Delete Confirmation Window
        if self.folder_delete_confirm.is_some() {
            let confirm_title = self.i18n.t("confirm_folder_delete_title");
            let confirm_msg = self.i18n.t("confirm_folder_delete");
            let cancel_text = self.i18n.t("cancel");
            
            let mut should_close = false;
            let mut delete_with_videos = false;
            let mut delete_folder_only = false;
            
            egui::Window::new(&confirm_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if let Some((folder_name, video_count)) = &self.folder_delete_confirm {
                        ui.label(format!("📁 {}", folder_name));
                        ui.separator();
                        ui.label(&confirm_msg);
                        
                        if *video_count > 0 {
                            ui.add_space(8.0);
                            let videos_msg = self.i18n.t("folder_contains_videos")
                                .replace("{}", &video_count.to_string());
                            ui.label(egui::RichText::new(videos_msg).color(egui::Color32::YELLOW));
                        }
                        
                        ui.add_space(16.0);
                        
                        ui.horizontal(|ui| {
                            if *video_count > 0 {
                                // Show both options when there are videos
                                if ui.button(egui::RichText::new(self.i18n.t("delete_videos_too")).color(egui::Color32::RED)).clicked() {
                                    delete_with_videos = true;
                                    should_close = true;
                                }
                                if ui.button(self.i18n.t("folder_only")).clicked() {
                                    delete_folder_only = true;
                                    should_close = true;
                                }
                            } else {
                                // No videos, just confirm folder removal
                                if ui.button("OK").clicked() {
                                    delete_folder_only = true;
                                    should_close = true;
                                }
                            }
                            if ui.button(&cancel_text).clicked() {
                                should_close = true;
                            }
                        });
                    }
                });
            
            // Process deletion after window is closed
            if should_close {
                if let Some((folder, _video_count)) = self.folder_delete_confirm.take() {
                    if delete_with_videos || delete_folder_only {
                        // Remove folder from list
                        self.database.folders.retain(|f| f != &folder);
                        
                        // Remove from watched_folders
                        self.watched_folders.retain(|path| {
                            path.file_name()
                                .and_then(|n| n.to_str())
                                .map(|name| name != folder)
                                .unwrap_or(true)
                        });
                        
                        if delete_with_videos {
                            // Also delete video profiles
                            self.database.videos.retain(|v| v.folder.as_ref() != Some(&folder));
                            eprintln!("[folder_management] Removed folder '{}' and its video profiles", folder);
                        } else {
                            eprintln!("[folder_management] Removed folder '{}' (kept video profiles)", folder);
                        }
                        
                        self.save_settings();
                        let _ = database::save_database(&self.database);
                    }
                }
            }
        }
        
        // Shader Management Window
        if self.show_shader_management_window {
            let shader_management_title = self.i18n.t("shader_management");
            let select_shader_text = self.i18n.t("select_shader_to_use");
            
            let mut window_open = self.show_shader_management_window;
            
            egui::Window::new(&shader_management_title)
                .open(&mut window_open)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label(&select_shader_text);
                    ui.separator();
                    
                    let mut shader_changed = false;
                    let available_shaders = video_player::get_available_shaders();
                    
                    if available_shaders.is_empty() {
                        ui.label("⚠ No shaders found in mpv/glsl_shaders directory");
                        ui.label("📁 Place .glsl shader files in the mpv/glsl_shaders directory");
                    } else {
                        // Add "None" option
                        if ui.radio(self.selected_shader.is_none(), "None").clicked() {
                            self.selected_shader = None;
                            shader_changed = true;
                        }
                        
                        ui.separator();
                        
                        // Add radio button for each shader
                        egui::ScrollArea::vertical()
                            .max_height(400.0)
                            .show(ui, |ui| {
                                for shader_name in &available_shaders {
                                    let is_selected = self.selected_shader.as_ref() == Some(shader_name);
                                    if ui.radio(is_selected, shader_name).clicked() {
                                        self.selected_shader = Some(shader_name.clone());
                                        shader_changed = true;
                                    }
                                }
                            });
                    }
                    
                    if shader_changed {
                        self.save_settings();
                    }
                });
            
            self.show_shader_management_window = window_open;
        }
        
        // Delete confirmation dialog
        if let Some(video_id) = self.delete_confirm_video.clone() {
            let is_multi = video_id == "__MULTI__";
            let title = if is_multi { 
                self.i18n.t("delete_selected_videos")
            } else { 
                self.i18n.t("delete_video")
            };
            
            egui::Window::new(&title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if is_multi {
                        let confirm_text = self.i18n.t("confirm_delete_videos").replace("{}", &self.selected_videos.len().to_string());
                        ui.label(&confirm_text);
                    } else {
                        ui.label(&self.i18n.t("confirm_delete_video"));
                        
                        if let Some(video) = self.database.get_video(&video_id) {
                            let title_text = self.i18n.t("title_label").replace("{}", &video.title);
                            ui.label(&title_text);
                        }
                    }
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button(&self.i18n.t("delete_keep_cache")).clicked() {
                            if is_multi {
                                self.delete_selected_videos(false);
                            } else {
                                self.delete_video(&video_id, false);
                            }
                            self.delete_confirm_video = None;
                        }
                        
                        if ui.button(&self.i18n.t("delete_remove_all")).clicked() {
                            if is_multi {
                                self.delete_selected_videos(true);
                            } else {
                                self.delete_video(&video_id, true);
                            }
                            self.delete_confirm_video = None;
                        }
                        
                        if ui.button(&self.i18n.t("cancel")).clicked() {
                            self.delete_confirm_video = None;
                        }
                    });
                });
        }
        
        // MPV shortcuts panel at bottom left
        if self.show_mpv_shortcuts {
            let mut window_open = self.mpv_shortcuts_open;
            let window_was_open = window_open;
            
            // Use saved position or default to bottom-left
            let default_pos = egui::pos2(10.0, ctx.screen_rect().max.y - 280.0);
            let initial_pos = self.mpv_shortcuts_pos.unwrap_or(default_pos);
            
            let response = egui::Window::new("MPV Shortcuts")
                .default_pos(initial_pos)
                .resizable(false)
                .collapsible(true)
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.set_width(180.0);
                    
                    ui.label(egui::RichText::new("⌨ Keyboard Shortcuts").strong());
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Space").code().strong());
                        ui.label("Play/Pause");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("F").code().strong());
                        ui.label("Toggle Fullscreen");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("←/→").code().strong());
                        ui.label("Seek -5/+5 sec");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("↑/↓").code().strong());
                        ui.label("Seek -60/+60 sec");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("9/0").code().strong());
                        ui.label("Volume Down/Up");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("M").code().strong());
                        ui.label("Mute/Unmute");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("[/]").code().strong());
                        ui.label("Speed -10%/+10%");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("{/}").code().strong());
                        ui.label("Speed x0.5/x2.0");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Q").code().strong());
                        ui.label("Quit Player");
                    });
                });
            
            // Track window position after drag
            if let Some(inner_response) = response {
                let new_pos = inner_response.response.rect.min;
                let pos_changed = self.mpv_shortcuts_pos.map(|p| (p.x - new_pos.x).abs() > 1.0 || (p.y - new_pos.y).abs() > 1.0).unwrap_or(true);
                if pos_changed {
                    self.mpv_shortcuts_pos = Some(new_pos);
                    self.save_settings();
                }
            }
            
            // Save state if changed
            if window_open != window_was_open {
                self.mpv_shortcuts_open = window_open;
                self.save_settings();
            } else {
                self.mpv_shortcuts_open = window_open;
            }
        }
        
        // Premium promotion window (shown when video limit reached)
        if self.show_premium_promotion_window {
            egui::Window::new(&self.i18n.t("premium_promotion_title"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_width(500.0);
                    
                    ui.label(egui::RichText::new(&self.i18n.t("premium_limit_reached")).size(16.0));
                    ui.add_space(10.0);
                    
                    ui.separator();
                    ui.add_space(5.0);
                    
                    ui.label(egui::RichText::new(&self.i18n.t("premium_unlock_features")).strong().size(14.0));
                    ui.add_space(5.0);
                    
                    // List premium features
                    ui.label(&self.i18n.t("premium_unlimited_storage"));
                    ui.label(&self.i18n.t("premium_scene_generation"));
                    ui.label(&self.i18n.t("premium_star_ratings"));
                    ui.label(&self.i18n.t("premium_glsl_shaders"));
                    ui.label(&self.i18n.t("premium_frame_interpolation"));
                    ui.label(&self.i18n.t("premium_gpu_rendering"));
                    ui.label(&self.i18n.t("premium_multi_select"));
                    
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);
                    
                    // Buttons
                    ui.horizontal(|ui| {
                        // License activation button
                        if ui.button(&self.i18n.t("enter_license_key")).clicked() {
                            self.show_license_window = true;
                            self.show_premium_promotion_window = false;
                        }
                        
                        // Close button
                        if ui.button(&self.i18n.t("got_it")).clicked() {
                            self.show_premium_promotion_window = false;
                        }
                    });
                });
        }
        
        // License activation window
        if self.show_license_window {
            let mut window_open = true;
            egui::Window::new(&self.i18n.t("activate_license"))
                .open(&mut window_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_width(500.0);
                    
                    ui.label(egui::RichText::new(&self.i18n.t("license_key_label")).strong());
                    ui.add_space(5.0);
                    
                    // License key input
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.license_input)
                            .desired_width(480.0)
                            .desired_rows(4)
                            .hint_text(&self.i18n.t("paste_license_key"))
                    );
                    
                    // Auto-focus input on first show
                    if response.has_focus() || response.gained_focus() {
                        // Input is focused
                    }
                    
                    ui.add_space(10.0);
                    
                    // Status message
                    if let Some(ref message) = self.license_status_message {
                        ui.label(egui::RichText::new(message).size(12.0));
                        ui.add_space(10.0);
                    }
                    
                    // Buttons
                    ui.horizontal(|ui| {
                        if ui.button(&self.i18n.t("activate")).clicked() {
                            let license_key = self.license_input.trim().to_string();
                            if !license_key.is_empty() {
                                self.activate_license(&license_key);
                            }
                        }
                        
                        if ui.button(&self.i18n.t("cancel")).clicked() {
                            self.show_license_window = false;
                            self.license_input.clear();
                            self.license_status_message = None;
                        }
                    });
                    
                    // Show current license info if available
                    if let Some(ref license) = self.current_license {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);
                        
                        ui.label(egui::RichText::new(&self.i18n.t("license_info")).strong());
                        ui.add_space(5.0);
                        
                        ui.horizontal(|ui| {
                            ui.label(&self.i18n.t("issued_to"));
                            ui.label(&license.info.issued_to);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(&self.i18n.t("expires"));
                            let expires_text = license.info.expires_at
                                .map(|ts| license::format_timestamp(ts))
                                .unwrap_or_else(|| self.i18n.t("never_expires"));
                            ui.label(&expires_text);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(&self.i18n.t("license_status"));
                            if license.is_expired {
                                ui.label(egui::RichText::new("❌ Expired").color(egui::Color32::RED));
                            } else if license.is_valid {
                                ui.label(egui::RichText::new("✅ Active").color(egui::Color32::GREEN));
                            } else {
                                ui.label(egui::RichText::new("❌ Invalid").color(egui::Color32::RED));
                            }
                        });
                    }
                });
            
            // Handle window close button (X)
            if !window_open {
                self.show_license_window = false;
                self.license_input.clear();
                self.license_status_message = None;
            }
        }
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save window state on exit (size and position only, not maximized state)
        let mut settings = database::load_settings().unwrap_or_default();
        
        settings.window_size = self.last_window_size;
        settings.window_position = self.last_window_pos;
        
        if let Err(e) = database::save_settings(&settings) {
            eprintln!("[app] Failed to save window state: {}", e);
        } else {
            eprintln!("[app] Window state saved: size={:?}, pos={:?}", 
                self.last_window_size, self.last_window_pos);
        }
    }
}

impl VideoPlayerApp {
    fn show_grid_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, videos: &[VideoFile]) {
        let available_width = ui.available_width();
        let base_item_width = 205.0;
        let item_width = base_item_width * self.thumbnail_scale;
        let spacing = 10.0;
        let items_per_row = ((available_width + spacing) / (item_width + spacing)).floor().max(1.0) as usize;
        
        for row_videos in videos.chunks(items_per_row) {
            ui.horizontal(|ui| {
                for video in row_videos {
                    ui.vertical(|ui| {
                        ui.set_width(item_width);
                        
                        let is_selected = self.selected_videos.contains(&video.id) || 
                                         self.selected_video.as_ref() == Some(&video.id);
                        
                        // Thumbnail with selection highlight
                        let base_thumbnail_size = egui::vec2(185.0, 142.0);
                        let thumbnail_size = base_thumbnail_size * self.thumbnail_scale;
                        
                        // Try to load actual thumbnail image
                        if let Some(ref thumb_path) = video.thumbnail_path {
                            let (rect, response) = ui.allocate_exact_size(thumbnail_size, egui::Sense::click());
                            
                            // Only load texture if the rect is actually visible (optimization)
                            if ui.is_rect_visible(rect) {
                                if let Some(texture) = self.load_image_texture(ctx, thumb_path) {
                                    // Draw thumbnail
                                    let _img_response = ui.put(rect, egui::Image::new(&texture).fit_to_exact_size(thumbnail_size));
                                    
                                    let painter = ui.painter();
                                    
                                    // Draw rating stars overlay in top-right corner
                                    if video.rating > 0 {
                                        let star_text = "★".repeat(video.rating as usize);
                                        let star_pos = egui::pos2(rect.max.x - 5.0, rect.min.y + 5.0);
                                        painter.text(
                                            star_pos,
                                            egui::Align2::RIGHT_TOP,
                                            star_text,
                                            egui::FontId::proportional(16.0 * self.thumbnail_scale),
                                            egui::Color32::from_rgb(255, 215, 0),
                                        );
                                    }
                                    
                                    // Draw selection highlight (blue border, on top of favorite if both)
                                    if is_selected {
                                        painter.rect_stroke(
                                            rect,
                                            4.0,
                                            egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 200, 255))
                                        );
                                    }
                                } else {
                                    // Fallback to placeholder if image can't be loaded
                                    ui.painter().rect_filled(rect, 4.0, egui::Color32::DARK_GRAY);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "🎬",
                                        egui::FontId::proportional(48.0 * self.thumbnail_scale),
                                        egui::Color32::WHITE,
                                    );
                                    
                                    if is_selected {
                                        ui.painter().rect_stroke(
                                            rect,
                                            4.0,
                                            egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 200, 255))
                                        );
                                    }
                                }
                            }
                            
                            // Handle click events (always handle regardless of visibility)
                            if response.clicked() {
                                let modifiers = ui.input(|i| i.modifiers.clone());
                                if modifiers.shift {
                                    // Shift+Click: range selection
                                    if let Some(ref last_selected) = self.last_selected_video.clone() {
                                        let video_ids: Vec<String> = videos.iter().map(|v| v.id.clone()).collect();
                                        self.select_range(&video_ids, last_selected, &video.id);
                                    } else {
                                        self.selected_videos.insert(video.id.clone());
                                    }
                                    self.last_selected_video = Some(video.id.clone());
                                    // Show profile of first selected video
                                    if self.selected_video.is_none() {
                                        self.selected_video = Some(video.id.clone());
                                        self.scene_panel_visible = true;
                                    }
                                } else if modifiers.ctrl {
                                    // Ctrl+Click: toggle selection
                                    self.toggle_video_selection(&video.id);
                                    self.last_selected_video = Some(video.id.clone());
                                    // Show profile of first selected video
                                    if self.selected_video.is_none() && self.selected_videos.contains(&video.id) {
                                        self.selected_video = Some(video.id.clone());
                                        self.scene_panel_visible = true;
                                    }
                                } else {
                                    // Single click: clear multi-select and select only this video
                                    self.selected_videos.clear();
                                    self.selected_video = Some(video.id.clone());
                                    self.scene_panel_visible = true;
                                    self.last_selected_video = Some(video.id.clone());
                                }
                            }
                            
                            // Double click: play video
                            if response.double_clicked() {
                                let selected_shader = self.selected_shader.as_deref();
                                let use_gpu_hq = self.is_premium && self.use_gpu_hq;
                                let use_custom_shaders = self.is_premium && self.use_custom_shaders;
                                let use_frame_interpolation = self.is_premium && self.use_frame_interpolation;
                                if let Err(e) = video_player::play_video_at_timestamp(&video.path, 0.0, self.mpv_always_on_top, use_gpu_hq, use_custom_shaders, selected_shader, use_frame_interpolation, self.mpv_volume) {
                                    eprintln!("Video playback error: {}", e);
                                }
                            }
                            
                            response.context_menu(|ui| {
                                if ui.button(&self.i18n.t("play_video")).clicked() {
                                    let selected_shader = self.selected_shader.as_deref();
                                    let use_gpu_hq = self.is_premium && self.use_gpu_hq;
                                    let use_custom_shaders = self.is_premium && self.use_custom_shaders;
                                    let use_frame_interpolation = self.is_premium && self.use_frame_interpolation;
                                    if let Err(e) = video_player::play_video_at_timestamp(&video.path, 0.0, self.mpv_always_on_top, use_gpu_hq, use_custom_shaders, selected_shader, use_frame_interpolation, self.mpv_volume) {
                                        eprintln!("Video playback error: {}", e);
                                    }
                                    ui.close_menu();
                                }
                                
                                ui.separator();
                                
                                if ui.button(&self.i18n.t("show_in_folder")).clicked() {
                                    if let Err(e) = video_player::show_in_folder(&video.path) {
                                        eprintln!("Show in folder error: {}", e);
                                    }
                                    ui.close_menu();
                                }
                                
                                ui.separator();
                                
                                if ui.button(&self.i18n.t("delete")).clicked() {
                                    self.delete_confirm_video = Some(video.id.clone());
                                    ui.close_menu();
                                }
                                
                                // Multiple selection delete option
                                if self.selected_videos.len() > 1 {
                                    ui.separator();
                                    
                                    let delete_multiple_text = self.i18n.t("delete_selected");
                                    if ui.button(&delete_multiple_text).clicked() {
                                        self.delete_confirm_video = Some("__MULTI__".to_string());
                                        ui.close_menu();
                                    }
                                }
                            });
                        } else {
                            // No thumbnail, show placeholder
                            self.draw_thumbnail_placeholder(ui, ctx, thumbnail_size, video);
                        }
                        
                        // Title
                        if self.show_full_filename {
                            ui.label(&video.title);
                        } else {
                            // Truncate if text exceeds available width
                            ui.add(egui::Label::new(&video.title).truncate());
                        }
                        
                        // Display tags if enabled
                        if self.show_tags_in_grid {
                            ui.horizontal(|ui| {
                                let video_id = video.id.clone();
                                for tag in video.tags.iter().take(2) {
                                    let tag_text = format!("#{}", tag);
                                    let tag_label = ui.label(tag_text);
                                    
                                    // Right-click on tag to remove
                                    tag_label.context_menu(|ui| {
                                        if ui.button("Remove Tag").clicked() {
                                            self.remove_tag_from_video(&video_id, tag);
                                            ui.close_menu();
                                        }
                                    });
                                }
                            });
                        }
                    });
                    
                    ui.add_space(spacing);
                }
            });
        }
    }
    
    fn show_list_view(&mut self, ui: &mut egui::Ui, videos: &[VideoFile]) {
        for video in videos {
            ui.horizontal(|ui| {
                let is_selected = self.selected_videos.contains(&video.id) || 
                                 self.selected_video.as_ref() == Some(&video.id);
                
                // Title with selection highlight
                let title_response = ui.selectable_label(is_selected, &video.title);
                if title_response.clicked() {
                    let modifiers = ui.input(|i| i.modifiers.clone());
                    if modifiers.shift {
                        // Shift+Click: range selection
                        if let Some(ref last_selected) = self.last_selected_video.clone() {
                            let video_ids: Vec<String> = videos.iter().map(|v| v.id.clone()).collect();
                            self.select_range(&video_ids, last_selected, &video.id);
                        } else {
                            self.selected_videos.insert(video.id.clone());
                        }
                        self.last_selected_video = Some(video.id.clone());
                        // Show profile of first selected video
                        if self.selected_video.is_none() {
                            self.selected_video = Some(video.id.clone());
                            self.scene_panel_visible = true;
                        }
                    } else if modifiers.ctrl {
                        self.toggle_video_selection(&video.id);
                        self.last_selected_video = Some(video.id.clone());
                        // Show profile of first selected video
                        if self.selected_video.is_none() && self.selected_videos.contains(&video.id) {
                            self.selected_video = Some(video.id.clone());
                            self.scene_panel_visible = true;
                        }
                    } else {
                        // Single click: clear multi-select and select only this video
                        self.selected_videos.clear();
                        self.selected_video = Some(video.id.clone());
                        self.scene_panel_visible = true;
                        self.last_selected_video = Some(video.id.clone());
                    }
                }
                
                // Double click: play video
                if title_response.double_clicked() {
                    let selected_shader = self.selected_shader.as_deref();
                    let use_gpu_hq = self.is_premium && self.use_gpu_hq;
                    let use_custom_shaders = self.is_premium && self.use_custom_shaders;
                    let use_frame_interpolation = self.is_premium && self.use_frame_interpolation;
                    if let Err(e) = video_player::play_video_at_timestamp(&video.path, 0.0, self.mpv_always_on_top, use_gpu_hq, use_custom_shaders, selected_shader, use_frame_interpolation, self.mpv_volume) {
                        eprintln!("Video playback error: {}", e);
                    }
                }
                
                // Tags
                let video_id = video.id.clone();
                for tag in &video.tags {
                    let tag_text = format!("#{}", tag);
                    let tag_label = ui.label(tag_text);
                    
                    // Right-click on tag to remove
                    tag_label.context_menu(|ui| {
                        if ui.button("Remove Tag").clicked() {
                            self.remove_tag_from_video(&video_id, tag);
                            ui.close_menu();
                        }
                    });
                }
                
                // Rating stars
                if video.rating > 0 {
                    let stars = "★".repeat(video.rating as usize) + &"☆".repeat((5 - video.rating) as usize);
                    ui.label(egui::RichText::new(stars).color(egui::Color32::from_rgb(255, 215, 0)));
                }
                
                // Folder
                if let Some(folder) = &video.folder {
                    ui.label(format!("Folder: {}", folder));
                }
                
                // File size
                let size_mb = video.file_size as f64 / 1024.0 / 1024.0;
                ui.label(format!("{:.1} MB", size_mb));
                
                // Delete button
                if ui.button("X").clicked() {
                    self.delete_confirm_video = Some(video.id.clone());
                }
            });
            
            ui.separator();
        }
    }
}

/// Load image data from file in a format suitable for creating a texture
/// Returns (rgba_pixels, [width, height]) or None if loading fails
fn load_image_data(image_path: &PathBuf) -> Option<(Vec<u8>, [usize; 2])> {
    let image_data = image::ImageReader::open(image_path)
        .ok()
        .and_then(|reader| reader.decode().ok())?;
    
    let size = [image_data.width() as usize, image_data.height() as usize];
    let image_buffer = image_data.to_rgba8();
    let pixels = image_buffer.into_raw();
    
    Some((pixels, size))
}
