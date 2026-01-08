use crate::models::VideoFile;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Generate folder name from file path
/// Uses the immediate parent directory name as the folder
pub fn generate_folder_from_path(path: &PathBuf) -> Option<String> {
    if let Some(parent) = path.parent() {
        if let Some(folder_name) = parent.file_name() {
            if let Some(name_str) = folder_name.to_str() {
                return Some(name_str.to_string());
            }
        }
    }
    None
}

/// Scan video files from directory
pub fn scan_directory(dir: PathBuf) -> Vec<VideoFile> {
    let mut videos = Vec::new();
    
    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path().to_path_buf();
        
        if path.is_file() && VideoFile::is_video_file(&path) {
            let mut video = VideoFile::new(path.clone());
            
            // Auto-generate folder from path
            video.folder = generate_folder_from_path(&path);
            
            // Get file size
            if let Ok(metadata) = std::fs::metadata(&path) {
                video.file_size = metadata.len();
            }
            
            videos.push(video);
        }
    }
    
    videos
}

