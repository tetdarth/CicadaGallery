use crate::models::VideoFile;
use crate::thumbnail;
use crate::scene_detection::{get_video_duration, get_video_resolution, get_video_frame_rate};
use std::path::PathBuf;
use walkdir::WalkDir;
use rayon::prelude::*;

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

/// Scan video files from directory (file path collection only - fast)
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

/// Process videos in parallel to generate thumbnails and metadata
/// This is the slow part that benefits from parallelization
pub fn process_videos_parallel(videos: Vec<VideoFile>, cache_dir: &PathBuf) -> Vec<VideoFile> {
    let cache_dir = cache_dir.clone();
    
    videos.into_par_iter()
        .map(|mut video| {
            // Generate thumbnail
            video.thumbnail_path = thumbnail::create_video_thumbnail(&video.path, &cache_dir);
            
            // Get video metadata using FFmpeg
            video.duration = get_video_duration(&video.path);
            video.resolution = get_video_resolution(&video.path);
            video.frame_rate = get_video_frame_rate(&video.path);
            
            video
        })
        .collect()
}

/// Process videos in parallel with a limit (for free tier)
pub fn process_videos_parallel_with_limit(
    videos: Vec<VideoFile>, 
    cache_dir: &PathBuf, 
    max_count: usize
) -> Vec<VideoFile> {
    let cache_dir = cache_dir.clone();
    let videos_to_process: Vec<_> = videos.into_iter().take(max_count).collect();
    
    videos_to_process.into_par_iter()
        .map(|mut video| {
            // Generate thumbnail
            video.thumbnail_path = thumbnail::create_video_thumbnail(&video.path, &cache_dir);
            
            // Get video metadata using FFmpeg
            video.duration = get_video_duration(&video.path);
            video.resolution = get_video_resolution(&video.path);
            video.frame_rate = get_video_frame_rate(&video.path);
            
            video
        })
        .collect()
}

/// Add a single file
pub fn add_single_file(path: PathBuf) -> Option<VideoFile> {
    if path.is_file() && VideoFile::is_video_file(&path) {
        let mut video = VideoFile::new(path.clone());
        
        // Auto-generate folder from path
        video.folder = generate_folder_from_path(&path);
        
        if let Ok(metadata) = std::fs::metadata(&path) {
            video.file_size = metadata.len();
        }
        
        Some(video)
    } else {
        None
    }
}
