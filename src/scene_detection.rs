use crate::models::{SceneInfo, VideoFile};
use crate::thumbnail::{get_ffmpeg_path, get_ffprobe_path};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

/// Detect scenes in a video using FFmpeg and generate thumbnails
pub fn detect_scenes(video: &mut VideoFile, cache_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create scene thumbnails directory
    let video_id = &video.id;
    let scene_dir = cache_dir.join("scenes").join(video_id);
    
    if !scene_dir.exists() {
        std::fs::create_dir_all(&scene_dir)?;
    }
    
    // Check if scenes already exist
    if !video.scenes.is_empty() {
        return Ok(());
    }
    
    // Use FFmpeg to detect scene changes and extract frames
    // This command detects scenes and outputs timestamps
    let mut cmd = Command::new(get_ffmpeg_path());
    cmd.args(&[
            "-i", video.path.to_str().unwrap(),
            "-filter:v", "select='gt(scene,0.3)',showinfo",
            "-vsync", "vfr",
            "-f", "null",
            "-"
        ]);
    
    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let output = cmd.output();
    
    if output.is_err() {
        // FFmpeg not available, create a few sample scenes at regular intervals
        return generate_interval_scenes(video, cache_dir);
    }
    
    let output = output?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Parse scene timestamps from FFmpeg output
    let mut timestamps: Vec<f64> = Vec::new();
    for line in stderr.lines() {
        if line.contains("pts_time:") {
            if let Some(time_str) = line.split("pts_time:").nth(1) {
                if let Some(time) = time_str.split_whitespace().next() {
                    if let Ok(timestamp) = time.parse::<f64>() {
                        timestamps.push(timestamp);
                    }
                }
            }
        }
    }
    
    // If no scenes detected or FFmpeg failed, use interval-based approach
    if timestamps.is_empty() {
        return generate_interval_scenes(video, cache_dir);
    }
    
    // Limit to reasonable number of scenes (e.g., max 50)
    timestamps.truncate(50);
    
    // Generate thumbnails for detected scenes in parallel
    generate_thumbnails_parallel(video, &timestamps, &scene_dir)?;
    
    Ok(())
}

/// Generate scene thumbnails at regular intervals (fallback when FFmpeg scene detection fails)
fn generate_interval_scenes(video: &mut VideoFile, cache_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let video_id = &video.id;
    let scene_dir = cache_dir.join("scenes").join(video_id);
    
    if !scene_dir.exists() {
        std::fs::create_dir_all(&scene_dir)?;
    }
    
    // Get video duration if available, otherwise estimate
    let duration = video.duration.unwrap_or(600.0); // Default to 10 minutes if unknown
    
    // Generate thumbnails every 30 seconds or divide into 20 parts, whichever is smaller
    let interval = (duration / 20.0).min(30.0).max(5.0);
    let num_scenes = (duration / interval).ceil() as usize;
    
    // Collect timestamps to generate
    let mut timestamps = Vec::new();
    for i in 0..num_scenes.min(50) {
        let timestamp = i as f64 * interval;
        if timestamp >= duration {
            break;
        }
        
        let thumbnail_path = scene_dir.join(format!("scene_{:03}.jpg", i));
        
        // Skip if already exists
        if thumbnail_path.exists() {
            video.scenes.push(SceneInfo {
                timestamp,
                thumbnail_path: thumbnail_path.clone(),
            });
        } else {
            timestamps.push(timestamp);
        }
    }
    
    // Generate missing thumbnails in parallel
    if !timestamps.is_empty() {
        generate_thumbnails_parallel(video, &timestamps, &scene_dir)?;
    }
    
    Ok(())
}

/// Generate thumbnails in parallel using multiple threads
fn generate_thumbnails_parallel(
    video: &mut VideoFile,
    timestamps: &[f64],
    scene_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine number of threads (use 4 for optimal balance between speed and resource usage)
    let num_threads = 4.min(timestamps.len());
    
    if num_threads == 0 {
        return Ok(());
    }
    
    let video_path = video.path.clone();
    let scene_dir = scene_dir.to_path_buf();
    let scenes = Arc::new(Mutex::new(Vec::new()));
    let ffmpeg_path = get_ffmpeg_path(); // Get path once before spawning threads
    
    // Split timestamps into chunks for each thread
    let chunk_size = (timestamps.len() + num_threads - 1) / num_threads;
    let mut handles = vec![];
    
    for (thread_id, chunk) in timestamps.chunks(chunk_size).enumerate() {
        let timestamps_chunk: Vec<f64> = chunk.to_vec();
        let video_path = video_path.clone();
        let scene_dir = scene_dir.clone();
        let scenes = Arc::clone(&scenes);
        let ffmpeg_path = ffmpeg_path.clone();
        
        let handle = thread::spawn(move || {
            let mut local_scenes = Vec::new();
            
            for (i, timestamp) in timestamps_chunk.iter().enumerate() {
                let global_index = thread_id * chunk_size + i;
                let thumbnail_path = scene_dir.join(format!("scene_{:03}.jpg", global_index));
                
                // Use optimized FFmpeg command: -ss before -i for fast seeking
                let mut cmd = Command::new(&ffmpeg_path);
                cmd.args(&[
                        "-ss", &timestamp.to_string(),  // Fast seek before input
                        "-i", video_path.to_str().unwrap(),
                        "-vframes", "1",
                        "-q:v", "3",  // Slightly lower quality for speed (was 2)
                        "-vf", "scale='min(320,iw)':-1",  // Limit size for faster processing
                        "-y",
                        thumbnail_path.to_str().unwrap()
                    ]);
                
                // Hide console window on Windows
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;
                    cmd.creation_flags(CREATE_NO_WINDOW);
                }
                
                let result = cmd.output();
                
                if result.is_ok() && thumbnail_path.exists() {
                    local_scenes.push(SceneInfo {
                        timestamp: *timestamp,
                        thumbnail_path,
                    });
                }
            }
            
            // Store results
            if let Ok(mut scenes) = scenes.lock() {
                scenes.extend(local_scenes);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        let _ = handle.join();
    }
    
    // Add all generated scenes to the video
    if let Ok(mut generated_scenes) = scenes.lock() {
        // Sort by timestamp
        generated_scenes.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
        video.scenes.extend(generated_scenes.drain(..));
    }
    
    Ok(())
}

/// Get video duration using FFprobe
pub fn get_video_duration(video_path: &Path) -> Option<f64> {
    let mut cmd = Command::new(get_ffprobe_path());
    cmd.args(&[
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            video_path.to_str()?
        ]);
    
    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let output = cmd.output().ok()?;
    
    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str.trim().parse::<f64>().ok()
}

/// Get video resolution (width, height) using FFprobe
pub fn get_video_resolution(video_path: &Path) -> Option<(u32, u32)> {
    let mut cmd = Command::new(get_ffprobe_path());
    cmd.args(&[
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width,height",
            "-of", "csv=p=0",
            video_path.to_str()?
        ]);
    
    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let output = cmd.output().ok()?;
    
    let resolution_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = resolution_str.trim().split(',').collect();
    
    if parts.len() == 2 {
        let width = parts[0].parse::<u32>().ok()?;
        let height = parts[1].parse::<u32>().ok()?;
        Some((width, height))
    } else {
        None
    }
}

/// Get video frame rate (fps) using FFprobe
pub fn get_video_frame_rate(video_path: &Path) -> Option<f64> {
    let mut cmd = Command::new(get_ffprobe_path());
    cmd.args(&[
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=r_frame_rate",
            "-of", "default=noprint_wrappers=1:nokey=1",
            video_path.to_str()?
        ]);
    
    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let output = cmd.output().ok()?;
    
    let fps_str = String::from_utf8_lossy(&output.stdout);
    let fps_str = fps_str.trim();
    
    // FFprobe returns frame rate as a fraction like "30000/1001" or "24/1"
    if let Some((num, den)) = fps_str.split_once('/') {
        let numerator = num.parse::<f64>().ok()?;
        let denominator = den.parse::<f64>().ok()?;
        if denominator > 0.0 {
            return Some(numerator / denominator);
        }
    }
    
    // Try parsing as a direct number
    fps_str.parse::<f64>().ok()
}

/// Format timestamp as HH:MM:SS
pub fn format_timestamp(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}

/// Add a single scene at a specific timestamp
/// Returns the created SceneInfo if successful
pub fn add_scene_at_timestamp(video: &mut VideoFile, timestamp: f64, cache_dir: &Path) -> Option<SceneInfo> {
    let video_id = &video.id;
    let scene_dir = cache_dir.join("scenes").join(video_id);
    
    if !scene_dir.exists() {
        std::fs::create_dir_all(&scene_dir).ok()?;
    }
    
    // Generate unique filename for the scene
    let scene_index = video.scenes.len();
    let thumbnail_path = scene_dir.join(format!("scene_manual_{:03}_{}.jpg", scene_index, (timestamp * 1000.0) as u64));
    
    // Use FFmpeg to extract the frame at the specified timestamp
    let ffmpeg_path = get_ffmpeg_path();
    let mut cmd = Command::new(&ffmpeg_path);
    cmd.args(&[
        "-ss", &timestamp.to_string(),
        "-i", video.path.to_str()?,
        "-vframes", "1",
        "-q:v", "2",
        "-vf", "scale='min(320,iw)':-1",
        "-y",
        thumbnail_path.to_str()?,
    ]);
    
    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let _ = cmd.output();
    
    if thumbnail_path.exists() {
        let scene = SceneInfo {
            timestamp,
            thumbnail_path: thumbnail_path.clone(),
        };
        
        // Insert scene in sorted order by timestamp
        let insert_pos = video.scenes.iter()
            .position(|s| s.timestamp > timestamp)
            .unwrap_or(video.scenes.len());
        video.scenes.insert(insert_pos, scene.clone());
        
        Some(scene)
    } else {
        None
    }
}
