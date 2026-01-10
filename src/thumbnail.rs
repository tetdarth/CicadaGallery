use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Get the path to the bundled ffmpeg executable
pub fn get_ffmpeg_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    
    eprintln!("[ffmpeg] exe_dir: {:?}", exe_dir);
    
    let bundled_ffmpeg = exe_dir.join("ffmpeg").join("bin").join("ffmpeg.exe");
    eprintln!("[ffmpeg] Looking for bundled ffmpeg at: {:?}", bundled_ffmpeg);
    
    if bundled_ffmpeg.exists() {
        eprintln!("[ffmpeg] Found bundled ffmpeg");
        return bundled_ffmpeg;
    }
    
    eprintln!("[ffmpeg] Bundled ffmpeg not found, falling back to system ffmpeg");
    // Fallback to system ffmpeg
    PathBuf::from("ffmpeg")
}

/// Get the path to the bundled ffprobe executable
pub fn get_ffprobe_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    
    let bundled_ffprobe = exe_dir.join("ffmpeg").join("bin").join("ffprobe.exe");
    eprintln!("[ffprobe] Looking for bundled ffprobe at: {:?}", bundled_ffprobe);
    
    if bundled_ffprobe.exists() {
        eprintln!("[ffprobe] Found bundled ffprobe");
        return bundled_ffprobe;
    }
    
    eprintln!("[ffprobe] Bundled ffprobe not found, falling back to system ffprobe");
    // Fallback to system ffprobe
    PathBuf::from("ffprobe")
}

/// サムネイルのキャッシュディレクトリを取得
pub fn get_cache_dir() -> PathBuf {
    let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    cache_dir.push("CicadaGallery");
    cache_dir.push("thumbnails");
    cache_dir
}

/// Generate a unique hash for a video path to avoid filename collisions
fn hash_path(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 動画ファイルからサムネイルパスを生成（実際の生成は後で実装）
pub fn create_video_thumbnail(video_path: &Path, cache_dir: &Path) -> Option<PathBuf> {
    // Use hash of full path to avoid collisions between same-named files in different folders
    let path_hash = hash_path(video_path);
    let thumbnail_path = cache_dir.join(format!("{}_thumb.jpg", path_hash));
    
    // サムネイルが既に存在する場合はそれを返す
    if thumbnail_path.exists() {
        return Some(thumbnail_path);
    }
    
    // キャッシュディレクトリが存在しない場合は作成
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir).ok()?;
    }
    
    // Try multiple seek positions for short videos
    // Try 5 seconds first, then 1 second, then 0.1 seconds (near start)
    let seek_positions = ["5.0", "1.0", "0.1"];
    
    for seek_pos in seek_positions {
        let mut cmd = Command::new(get_ffmpeg_path());
        cmd.args(&[
                "-ss", seek_pos,  // Seek position
                "-i", video_path.to_str()?,
                "-vframes", "1",  // Extract one frame
                "-q:v", "2",  // High quality
                "-vf", "scale=320:-1",  // Scale to width 320, maintain aspect ratio
                "-y",  // Overwrite output file
                thumbnail_path.to_str()?
            ]);
        
        // Hide console window on Windows
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        
        let _ = cmd.output();
        
        // Check if FFmpeg succeeded
        if thumbnail_path.exists() {
            // Verify the file is not empty
            if let Ok(metadata) = std::fs::metadata(&thumbnail_path) {
                if metadata.len() > 0 {
                    return Some(thumbnail_path);
                }
            }
            // If file is empty, remove it and try next position
            let _ = std::fs::remove_file(&thumbnail_path);
        }
    }
    
    None
}

