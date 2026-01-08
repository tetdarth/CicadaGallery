use std::path::Path;
use std::thread;
use std::process::{Command, Stdio};
use std::io::Write;
use std::fs::OpenOptions;

fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug.log")
    {
        let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%H:%M:%S"), msg);
    }
    eprintln!("{}", msg);
}

/// Shared video player state using mpv subprocess
#[derive(Clone)]
pub struct VideoPlayer {}

impl VideoPlayer {
    /// Create a new video player instance
    pub fn new() -> Self {
        Self {}
    }
    
    /// Play a video file at a specific timestamp in a separate window
    pub fn play_video(
        &self,
        video_path: &Path,
        timestamp_seconds: f64,
        always_on_top: bool,
        use_gpu_hq: bool,
        use_custom_shaders: bool,
        selected_shader: Option<&str>,
        use_frame_interpolation: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log_debug(&format!("[DEBUG] play_video called for: {:?}", video_path));
        
        let video_path = video_path.to_path_buf();
        let shader = selected_shader.map(|s| s.to_string());
        
        // Spawn a new thread for the player
        thread::spawn(move || {
            log_debug("[DEBUG] Thread spawned, calling run_player");
            if let Err(e) = Self::run_player(
                &video_path,
                timestamp_seconds,
                always_on_top,
                use_gpu_hq,
                use_custom_shaders,
                shader.as_deref(),
                use_frame_interpolation,
            ) {
                log_debug(&format!("[ERROR] Error playing video: {}", e));
            }
            log_debug("[DEBUG] Thread exiting");
        });
        
        log_debug("[DEBUG] play_video returning");
        Ok(())
    }
    
    fn run_player(
        video_path: &Path,
        timestamp_seconds: f64,
        always_on_top: bool,
        use_gpu_hq: bool,
        use_custom_shaders: bool,
        selected_shader: Option<&str>,
        use_frame_interpolation: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log_debug(&format!("[DEBUG] Starting run_player for: {:?}", video_path));
        
        // Look for mpv executable
        let mpv_paths = vec![
            "mpv\\mpv.exe",
            "mpv.exe",
            ".\\mpv\\mpv.exe",
            ".\\mpv.exe",
        ];
        
        let mut mpv_path = None;
        for path in &mpv_paths {
            if std::path::Path::new(path).exists() {
                mpv_path = Some(*path);
                log_debug(&format!("[DEBUG] Found mpv at: {}", path));
                break;
            }
        }
        
        let mpv_exe = mpv_path.ok_or("mpv.exe not found. Please place mpv.exe in the project directory or mpv folder.")?;
        
        // Build mpv command
        let mut cmd = Command::new(mpv_exe);
        
        // Basic options
        cmd.arg("--keep-open=yes");
        cmd.arg("--osc=yes");
        cmd.arg("--input-default-bindings=yes");
        cmd.arg("--no-initial-audio-sync"); // Reduce startup delay
        
        // Video output
        cmd.arg("--vo=gpu");
        cmd.arg("--gpu-api=d3d11");
        
        // Hardware decoding
        cmd.arg("--hwdec=auto");
        
        // Always on top
        if always_on_top {
            cmd.arg("--ontop=yes");
        }
        
        // GPU HQ profile
        if use_gpu_hq {
            cmd.arg("--profile=gpu-hq");
            cmd.arg("--scale=ewa_lanczossharp");
            cmd.arg("--cscale=ewa_lanczossharp");
            cmd.arg("--deband=yes");
        }
        
        // Frame interpolation
        if use_frame_interpolation {
            cmd.arg("--interpolation=yes");
            cmd.arg("--tscale=oversample");
            cmd.arg("--video-sync=display-resample");
        }
        
        // Custom shaders
        if use_custom_shaders {
            if let Some(shader_path) = get_shader_path(selected_shader) {
                cmd.arg(format!("--glsl-shaders={}", shader_path));
            }
        }
        
        // Start position
        if timestamp_seconds > 0.0 {
            cmd.arg(format!("--start={}", timestamp_seconds));
        }
        
        // Video file
        cmd.arg("--");
        cmd.arg(video_path);
        
        log_debug(&format!("[DEBUG] Executing mpv command: {:?}", cmd));
        
        // Spawn mpv process with piped stdin
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        
        log_debug(&format!("[DEBUG] mpv process spawned with PID: {}", child.id()));
        
        // Wait for mpv to complete (this is the blocking call that keeps the window open)
        let _status = child.wait()?;
        log_debug("[DEBUG] mpv process exited");
        
        Ok(())
    }
}

/// Play a video at a specific timestamp (wrapper function)
pub fn play_video_at_timestamp(
    video_path: &Path,
    timestamp_seconds: f64,
    always_on_top: bool,
    use_gpu_hq: bool,
    use_custom_shaders: bool,
    selected_shader: Option<&str>,
    use_frame_interpolation: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log_debug(&format!("[DEBUG] play_video_at_timestamp called for: {:?} at {:.2}s", video_path, timestamp_seconds));
    
    let player = VideoPlayer::new();
    player.play_video(
        video_path,
        timestamp_seconds,
        always_on_top,
        use_gpu_hq,
        use_custom_shaders,
        selected_shader,
        use_frame_interpolation,
    )?;
    
    log_debug("[DEBUG] play_video_at_timestamp completed");
    Ok(())
}

/// Open the containing folder and select the file
pub fn show_in_folder(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(&["/select,", path.to_str().unwrap()])
            .spawn()?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        let parent = path.parent().unwrap_or(path);
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(parent).spawn()?;
        }
        
        #[cfg(target_os = "linux")]
        {
            if Command::new("nautilus").arg(parent).spawn().is_err() {
                Command::new("xdg-open").arg(parent).spawn()?;
            }
        }
    }
    
    Ok(())
}

/// Get list of available shader files
pub fn get_available_shaders() -> Vec<String> {
    vec![
        "Anime4K".to_string(),
        "FSRCNNX".to_string(),
        "NNEDI3".to_string(),
        "NVScaler".to_string(),
        "KrigBilateral".to_string(),
        "SSimDownscaler".to_string(),
    ]
}

fn get_shader_path(shader_name: Option<&str>) -> Option<String> {
    shader_name.map(|name| {
        let shader_file = match name {
            "Anime4K" => "Anime4K_Upscale_CNN_x2_VL.glsl",
            "FSRCNNX" => "FSRCNNX_x2_8-0-4-1.glsl",
            "NNEDI3" => "nnedi3-nns64-win8x6.hook",
            "NVScaler" => "NVScaler.glsl",
            "KrigBilateral" => "KrigBilateral.glsl",
            "SSimDownscaler" => "SSimDownscaler.glsl",
            _ => return format!("glsl_shaders\\{}", name),
        };
        format!("glsl_shaders\\{}", shader_file)
    })
}
