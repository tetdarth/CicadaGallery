use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use crate::i18n::Language;

/// Scene information for a video
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInfo {
    pub timestamp: f64, // timestamp in seconds
    pub thumbnail_path: PathBuf,
}

/// Video file information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    pub id: String,
    pub path: PathBuf,
    pub title: String,
    pub duration: Option<f64>, // in seconds
    pub file_size: u64,
    pub resolution: Option<(u32, u32)>, // (width, height)
    pub thumbnail_path: Option<PathBuf>,
    pub tags: Vec<String>,
    pub folder: Option<String>,
    #[serde(default)]
    pub rating: u8, // 0-5 star rating (0 = no rating, 1-5 = stars)
    #[serde(default, skip_serializing, rename = "is_favorite")]
    pub is_favorite_legacy: Option<bool>, // For backward compatibility during deserialization
    pub added_date: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub scenes: Vec<SceneInfo>, // Scene thumbnails and timestamps
}

impl VideoFile {
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let id = uuid::Uuid::new_v4().to_string();
        
        // Try to get file creation time, fall back to modified time, then to current time
        let added_date = std::fs::metadata(&path)
            .ok()
            .and_then(|metadata| {
                metadata.created()
                    .or_else(|_| metadata.modified())
                    .ok()
            })
            .and_then(|system_time| {
                use std::time::UNIX_EPOCH;
                system_time.duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|duration| {
                        DateTime::<Utc>::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
                            .unwrap_or_else(|| Utc::now())
                    })
            })
            .unwrap_or_else(|| Utc::now());
        
        Self {
            id,
            path,
            title,
            duration: None,
            file_size: 0,
            resolution: None,
            thumbnail_path: None,
            tags: Vec::new(),
            folder: None,
            rating: 0,
            is_favorite_legacy: None,
            added_date,
            last_played: None,
            scenes: Vec::new(),
        }
    }
    
    pub fn is_video_file(path: &PathBuf) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(
                ext.as_str(),
                "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
            )
        } else {
            false
        }
    }
}

/// Application database
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoDatabase {
    pub videos: Vec<VideoFile>,
    pub folders: Vec<String>,
    pub tags: Vec<String>,
}

impl VideoDatabase {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_video(&mut self, video: VideoFile) {
        // Add folder to folders list if it doesn't exist
        if let Some(ref folder) = video.folder {
            self.add_folder(folder.clone());
        }
        self.videos.push(video);
    }
    
    pub fn remove_video(&mut self, id: &str) {
        self.videos.retain(|v| v.id != id);
    }
    
    pub fn get_video(&self, id: &str) -> Option<&VideoFile> {
        self.videos.iter().find(|v| v.id == id)
    }
    
    pub fn get_video_mut(&mut self, id: &str) -> Option<&mut VideoFile> {
        self.videos.iter_mut().find(|v| v.id == id)
    }
    
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
    
    pub fn add_folder(&mut self, folder: String) {
        if !self.folders.contains(&folder) {
            self.folders.push(folder);
        }
    }
    
    pub fn get_by_rating(&self, min_rating: u8) -> Vec<&VideoFile> {
        self.videos.iter().filter(|v| v.rating >= min_rating).collect()
    }
    
    pub fn get_by_folder(&self, folder: &str) -> Vec<&VideoFile> {
        self.videos
            .iter()
            .filter(|v| v.folder.as_deref() == Some(folder))
            .collect()
    }
    
    pub fn get_by_tag(&self, tag: &str) -> Vec<&VideoFile> {
        self.videos
            .iter()
            .filter(|v| v.tags.contains(&tag.to_string()))
            .collect()
    }
    
    /// Check if a video path already exists in the database
    pub fn has_video_path(&self, path: &std::path::PathBuf) -> bool {
        self.videos.iter().any(|v| v.path == *path)
    }
    
    /// Get a mutable reference to a video by its path
    pub fn get_video_by_path_mut(&mut self, path: &std::path::PathBuf) -> Option<&mut VideoFile> {
        // Try to canonicalize both paths for comparison
        let canonical_path = path.canonicalize().ok();
        
        self.videos.iter_mut().find(|v| {
            if let Some(ref target) = canonical_path {
                if let Ok(v_canonical) = v.path.canonicalize() {
                    return v_canonical == *target;
                }
            }
            // Fallback to direct comparison if canonicalization fails
            v.path == *path
        })
    }
    
    /// Get all unique parent folders from video paths
    pub fn get_scanned_folders(&self) -> std::collections::HashSet<std::path::PathBuf> {
        self.videos
            .iter()
            .filter_map(|v| v.path.parent().map(|p| p.to_path_buf()))
            .collect()
    }
    
    /// Remove duplicate videos from the database (keep the first occurrence)
    pub fn remove_duplicates(&mut self) -> usize {
        use std::collections::HashMap;
        
        let mut seen_paths: HashMap<std::path::PathBuf, usize> = HashMap::new();
        let mut duplicates_indices = Vec::new();
        
        for (idx, video) in self.videos.iter().enumerate() {
            // Try to canonicalize the path
            let key = video.path.canonicalize().unwrap_or_else(|_| video.path.clone());
            
            if let Some(&first_idx) = seen_paths.get(&key) {
                // This is a duplicate, mark for removal
                eprintln!("[remove_duplicates] Found duplicate: {:?} (keeping index {}, removing {})", video.path, first_idx, idx);
                duplicates_indices.push(idx);
            } else {
                seen_paths.insert(key, idx);
            }
        }
        
        // Remove duplicates in reverse order to maintain correct indices
        let count = duplicates_indices.len();
        for idx in duplicates_indices.iter().rev() {
            self.videos.remove(*idx);
        }
        
        count
    }
    
    /// Remove unused tags from the database
    /// Tags that are not used by any video will be removed
    pub fn cleanup_unused_tags(&mut self) {
        let used_tags: std::collections::HashSet<String> = self.videos
            .iter()
            .flat_map(|v| v.tags.iter().cloned())
            .collect();
        
        self.tags.retain(|tag| used_tags.contains(tag));
    }
    
    /// Remove unused folders from the database
    /// Folders that are not used by any video will be removed
    pub fn cleanup_unused_folders(&mut self) {
        let used_folders: std::collections::HashSet<String> = self.videos
            .iter()
            .filter_map(|v| v.folder.clone())
            .collect();
        
        self.folders.retain(|folder| used_folders.contains(folder));
    }
    
    /// Update added_date for all videos based on file metadata
    /// Uses file creation time if available, otherwise uses modified time
    pub fn update_added_dates_from_files(&mut self) {
        use std::time::UNIX_EPOCH;
        
        for video in &mut self.videos {
            if let Ok(metadata) = std::fs::metadata(&video.path) {
                let file_time = metadata.created()
                    .or_else(|_| metadata.modified())
                    .ok();
                
                if let Some(system_time) = file_time {
                    if let Ok(duration) = system_time.duration_since(UNIX_EPOCH) {
                        if let Some(datetime) = DateTime::<Utc>::from_timestamp(
                            duration.as_secs() as i64,
                            duration.subsec_nanos()
                        ) {
                            video.added_date = datetime;
                        }
                    }
                }
            }
        }
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub thumbnail_scale: f32,
    pub mpv_always_on_top: bool,
    pub show_full_filename: bool,
    pub show_tags_in_grid: bool,
    pub dark_mode: bool,
    pub use_gpu_hq: bool,
    pub use_custom_shaders: bool,
    pub selected_shader: Option<String>, // Selected shader filename
    pub use_frame_interpolation: bool,
    pub language: Language, // UI language
    #[serde(default)]
    pub added_dates_updated: bool, // Flag to track if added_date has been updated from file metadata
    #[serde(default)]
    pub watched_folders: Vec<std::path::PathBuf>, // Folders being watched for changes
    #[serde(default = "default_mpv_shortcuts_open")]
    pub mpv_shortcuts_open: bool, // MPV shortcuts panel open/collapsed state
    #[serde(default = "default_mpv_volume")]
    pub mpv_volume: u8, // MPV volume (0-100)
    #[serde(default)]
    pub license_key: Option<String>, // Signed license key
}

fn default_mpv_shortcuts_open() -> bool {
    true
}

fn default_mpv_volume() -> u8 {
    100
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            thumbnail_scale: 1.0,
            mpv_always_on_top: true,
            show_full_filename: false,
            show_tags_in_grid: true,
            dark_mode: false,
            use_gpu_hq: false,
            use_custom_shaders: false,
            selected_shader: None,
            use_frame_interpolation: false,
            language: Language::default(),
            added_dates_updated: false,
            watched_folders: Vec::new(),
            mpv_shortcuts_open: true,
            mpv_volume: 100,
            license_key: None,
        }
    }
}
