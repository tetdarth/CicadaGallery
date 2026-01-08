use std::path::{Path, PathBuf};
use std::process::Command;

/// サムネイルのキャッシュディレクトリを取得
pub fn get_cache_dir() -> PathBuf {
    let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    cache_dir.push("cicadaGallaley");
    cache_dir.push("thumbnails");
    cache_dir
}

/// 動画ファイルからサムネイルパスを生成（実際の生成は後で実装）
pub fn create_video_thumbnail(video_path: &Path, cache_dir: &Path) -> Option<PathBuf> {
    // ビデオファイル名からサムネイル名を生成
    let video_name = video_path.file_stem()?.to_str()?;
    let thumbnail_path = cache_dir.join(format!("{}_thumb.jpg", video_name));
    
    // サムネイルが既に存在する場合はそれを返す
    if thumbnail_path.exists() {
        return Some(thumbnail_path);
    }
    
    // キャッシュディレクトリが存在しない場合は作成
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir).ok()?;
    }
    
    // Generate thumbnail using FFmpeg at 5 seconds position
    let mut cmd = Command::new("ffmpeg");
    cmd.args(&[
            "-ss", "5.0",  // Seek to 5 seconds
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
    
    let result = cmd.output();
    
    // Check if FFmpeg succeeded
    if result.is_ok() && thumbnail_path.exists() {
        Some(thumbnail_path)
    } else {
        None
    }
}

