use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;
use std::fs::OpenOptions;

const IPC_PIPE_NAME: &str = "\\\\.\\pipe\\mpv-cicada-ipc";

/// Get the directory where the executable is located
fn get_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// Get all available GLSL shader files from mpv/glsl_shaders directory
pub fn get_available_shaders() -> Vec<String> {
    let mut shaders = Vec::new();
    
    // Try EXE directory first, then fall back to relative paths
    let mut shader_dirs: Vec<PathBuf> = Vec::new();
    
    if let Some(exe_dir) = get_exe_dir() {
        shader_dirs.push(exe_dir.join("mpv").join("glsl_shaders"));
    }
    
    // Fallback to relative paths (for development)
    shader_dirs.extend([
        PathBuf::from("mpv\\glsl_shaders"),
        PathBuf::from(".\\mpv\\glsl_shaders"),
        PathBuf::from("..\\mpv\\glsl_shaders"),
    ]);
    
    for shader_dir in &shader_dirs {
        if shader_dir.exists() && shader_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(shader_dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if let Some(ext) = entry_path.extension() {
                        if ext == "glsl" || ext == "hook" {
                            if let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) {
                                shaders.push(filename.to_string());
                            }
                        }
                    }
                }
            }
            
            // If we found shaders in this directory, stop searching
            if !shaders.is_empty() {
                eprintln!("[shader] Found shaders in: {:?}", shader_dir);
                break;
            }
        }
    }
    
    shaders.sort();
    shaders
}

/// Get GLSL shader file paths for enabled shaders (absolute path for reliability)
fn get_shader_files(selected_shader: Option<&str>) -> Vec<String> {
    let mut shader_paths = Vec::new();
    
    if let Some(shader_name) = selected_shader {
        // Try EXE directory first, then fall back to relative paths
        let mut shader_dirs: Vec<PathBuf> = Vec::new();
        
        if let Some(exe_dir) = get_exe_dir() {
            shader_dirs.push(exe_dir.join("mpv").join("glsl_shaders"));
        }
        
        // Fallback to relative paths (for development)
        shader_dirs.extend([
            PathBuf::from("mpv\\glsl_shaders"),
            PathBuf::from(".\\mpv\\glsl_shaders"),
            PathBuf::from("..\\mpv\\glsl_shaders"),
        ]);
        
        for shader_dir in &shader_dirs {
            if shader_dir.exists() && shader_dir.is_dir() {
                let shader_path = shader_dir.join(shader_name);
                
                if shader_path.exists() {
                    // Use absolute path for reliability
                    if let Ok(absolute_path) = shader_path.canonicalize() {
                        if let Some(path_str) = absolute_path.to_str() {
                            // Remove \\?\ prefix on Windows
                            let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(path_str);
                            shader_paths.push(clean_path.to_string());
                            eprintln!("[shader] Using shader: {}", clean_path);
                        }
                    }
                    break;
                }
            }
        }
    }
    
    shader_paths
}

/// Get mpv executable path (tries EXE directory first, then relative paths)
fn get_mpv_path() -> Option<PathBuf> {
    // Try EXE directory first
    if let Some(exe_dir) = get_exe_dir() {
        let mpv_path = exe_dir.join("mpv").join("mpv.exe");
        if mpv_path.exists() {
            eprintln!("[mpv] Found mpv at: {:?}", mpv_path);
            return Some(mpv_path);
        }
    }
    
    // Fallback to relative paths (for development)
    let relative_paths = [
        "mpv\\mpv.exe",
        ".\\mpv\\mpv.exe",
        "..\\mpv\\mpv.exe",
    ];
    
    for path_str in &relative_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            eprintln!("[mpv] Found mpv at: {:?}", path);
            return Some(path);
        }
    }
    
    None
}

/// Send IPC command to existing mpv instance
fn send_ipc_command(video_path: &str, timestamp_seconds: f64) -> bool {
    eprintln!("IPC通信を試行: パイプ={}", IPC_PIPE_NAME);
    
    // 名前付きパイプに接続を試みる
    let pipe_result = OpenOptions::new()
        .write(true)
        .read(false)
        .open(IPC_PIPE_NAME);
    
    if let Ok(mut pipe) = pipe_result {
        eprintln!("IPC接続成功。動画を読み込みます: {}, {}秒", video_path, timestamp_seconds);
        
        // loadfileコマンドでstart時刻をオプションとして指定
        let load_cmd = format!(
            "{{\"command\":[\"loadfile\",\"{}\",\"replace\",\"0\",\"start={}\"]}}\n",
            video_path.replace("\\", "\\\\"),
            timestamp_seconds
        );
        
        if pipe.write_all(load_cmd.as_bytes()).is_err() {
            eprintln!("loadfileコマンド送信失敗");
            return false;
        }
        
        // 少し待機してから明示的にseek
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        let seek_cmd = format!(
            "{{\"command\":[\"seek\",{},\"absolute\"]}}\n",
            timestamp_seconds
        );
        
        if pipe.write_all(seek_cmd.as_bytes()).is_err() {
            eprintln!("seekコマンド送信失敗");
            // seekが失敗しても続行
        }
        
        eprintln!("✓ 既存のmpvインスタンスにコマンドを送信しました: {}秒", timestamp_seconds);
        return true;
    }
    
    eprintln!("IPC接続失敗（既存インスタンスなし）");
    false
}

/// Helper function to try running a command and check if it succeeds
fn try_play_video(command: &str, args: &[&str]) -> bool {
    let mut cmd = Command::new(command);
    cmd.args(args);
    
    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    match cmd.spawn()
    {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Failed to launch {}: {}", command, e);
            false
        }
    }
}

/// Play video with timestamp using available video players
/// Tries mpv.net, mpv, VLC, then falls back to system default player
pub fn play_video_at_timestamp(video_path: &Path, timestamp_seconds: f64, always_on_top: bool, use_gpu_hq: bool, use_custom_shaders: bool, selected_shader: Option<&str>, use_frame_interpolation: bool, volume: u8) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_str = video_path.to_str().unwrap();
    
    // 既存のmpvインスタンスにコマンドを送信
    if send_ipc_command(video_path_str, timestamp_seconds) {
        return Ok(());
    }
    
    // 既存のインスタンスがない場合は新規起動
    let start_arg = format!("--start={}", timestamp_seconds);
    let volume_arg = format!("--volume={}", volume);
    let ipc_arg = format!("--input-ipc-server={}", IPC_PIPE_NAME);
    let ontop_arg = "--ontop";
    let log_arg = "--log-file=mpv.log";
    
    // Collect shader files if custom shaders are enabled
    let shader_files = if use_custom_shaders {
        get_shader_files(selected_shader)
    } else {
        Vec::new()
    };
    
    // Build shader arguments
    let shader_args: Vec<String> = shader_files.iter()
        .map(|shader| format!("--glsl-shaders={}", shader))
        .collect();
    
    // GPU high-quality options (vo and gpu-api are added separately)
    let gpu_hq_args = [
        "--profile=gpu-hq",
        "--scale=ewa_lanczossharp",
        "--cscale=ewa_lanczossharp",
        "--deband=yes",
        "--deband-iterations=2",
        "--deband-threshold=35",
        "--deband-range=16",
        "--deband-grain=8",
    ];
    
    // Frame interpolation options
    let frame_interpolation_args = [
        "--interpolation=yes",
        "--tscale=oversample",
        "--video-sync=display-resample",
    ];
    
    #[cfg(target_os = "windows")]
    {
        // Try to find mpv.exe using get_mpv_path (EXE dir first, then relative)
        if let Some(mpv_path) = get_mpv_path() {
            // Build arguments based on options
            let mut args: Vec<String> = vec![ipc_arg.clone(), start_arg.clone(), volume_arg.clone(), log_arg.to_string()];
            
            // Enable GPU rendering if using gpu-hq or custom shaders
            if use_gpu_hq || (use_custom_shaders && !shader_args.is_empty()) {
                args.push("--vo=gpu-next".to_string());
                args.push("--gpu-api=d3d11".to_string());
            }
            
            if use_gpu_hq {
                args.extend(gpu_hq_args.iter().map(|s| s.to_string()));
            }
            
            if use_frame_interpolation {
                args.extend(frame_interpolation_args.iter().map(|s| s.to_string()));
            }
            
            // Add custom shader arguments
            args.extend(shader_args.clone());
                
                if always_on_top {
                    args.push(ontop_arg.to_string());
                }
                
                args.push(video_path_str.to_string());
                
                let mpv_path_str = mpv_path.to_str().unwrap_or("mpv.exe");
                eprintln!("🚀 Launching mpv: {} {:?}", mpv_path_str, args);
                
                // Convert to &str refs
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                
                if try_play_video(mpv_path_str, &args_refs) {
                    return Ok(());
                }
        }
        
        // Try system mpv.exe
        let mut args: Vec<String> = vec![ipc_arg.clone(), start_arg.clone()];
        
        // Enable GPU rendering if using gpu-hq or custom shaders
        if use_gpu_hq || (use_custom_shaders && !shader_args.is_empty()) {
            args.push("--vo=gpu-next".to_string());
            args.push("--gpu-api=d3d11".to_string());
        }
        
        if use_gpu_hq {
            args.extend(gpu_hq_args.iter().map(|s| s.to_string()));
        }
        
        if use_frame_interpolation {
            args.extend(frame_interpolation_args.iter().map(|s| s.to_string()));
        }
        
        // Add custom shader arguments
        args.extend(shader_args.clone());
        
        if always_on_top {
            args.push(ontop_arg.to_string());
        }
        
        args.push(video_path_str.to_string());
        
        // Convert to &str refs
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        
        if try_play_video("mpv", &args_refs) {
            return Ok(());
        }
        
        // Try VLC
        // Check common VLC installation paths
        let vlc_paths = [
            "C:\\Program Files\\VideoLAN\\VLC\\vlc.exe",
            "C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe",
        ];
        
        for vlc_path in &vlc_paths {
            if std::path::Path::new(vlc_path).exists() {
                if let Ok(_) = Command::new(vlc_path)
                    .arg(format!("--start-time={}", timestamp_seconds as i64))
                    .arg(video_path)
                    .spawn()
                {
                    return Ok(());
                }
            }
        }
        
        // Fallback to default player (no timestamp support)
        play_video(video_path)?;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Try mpv first
        if let Ok(_) = Command::new("mpv")
            .arg(format!("--start={}", timestamp_seconds))
            .arg(video_path)
            .spawn()
        {
            return Ok(());
        }
        
        // Try VLC
        if let Ok(_) = Command::new("vlc")
            .arg(format!("--start-time={}", timestamp_seconds as i64))
            .arg(video_path)
            .spawn()
        {
            return Ok(());
        }
        
        // Fallback to default player
        play_video(video_path)?;
    }
    
    #[cfg(target_os = "macos")]
    {
        // Try mpv first
        if let Ok(_) = Command::new("mpv")
            .arg(format!("--start={}", timestamp_seconds))
            .arg(video_path)
            .spawn()
        {
            return Ok(());
        }
        
        // Try VLC
        if let Ok(_) = Command::new("/Applications/VLC.app/Contents/MacOS/VLC")
            .arg(format!("--start-time={}", timestamp_seconds as i64))
            .arg(video_path)
            .spawn()
        {
            return Ok(());
        }
        
        // Fallback to default player
        play_video(video_path)?;
    }
    
    Ok(())
}

/// Play video with mpv.net/mpv if available, otherwise system default player
#[cfg(target_os = "windows")]
pub fn play_video(video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let video_path_str = video_path.to_str().unwrap();
    
    // Try local mpvnet first (in project folder)
    let local_mpvnet_paths = [
        "mpv.net-v7.1.1.0-portable\\mpvnet.exe",
        ".\\mpv.net-v7.1.1.0-portable\\mpvnet.exe",
        "mpvnet.exe",
        ".\\mpvnet.exe",
    ];
    
    for mpvnet_path in &local_mpvnet_paths {
        if std::path::Path::new(mpvnet_path).exists() {
            if try_play_video(mpvnet_path, &["--no-single-instance", video_path_str]) {
                return Ok(());
            }
        }
    }
    
    // Try mpvnet from system PATH
    let mpvnet_locations = [
        "mpvnet",
        "C:\\Program Files\\mpv.net\\mpvnet.exe",
        "C:\\Program Files (x86)\\mpv.net\\mpvnet.exe",
    ];
    
    for mpvnet in &mpvnet_locations {
        if try_play_video(mpvnet, &["--no-single-instance", video_path_str]) {
            return Ok(());
        }
    }
    
    // Try mpv
    if try_play_video("mpv", &[video_path_str]) {
        return Ok(());
    }
    
    // Fallback: Windows default player
    let mut cmd = Command::new("cmd");
    cmd.args(&["/C", "start", "", video_path_str]);
    
    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    cmd.spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn play_video(video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Linux: Use xdg-open
    Command::new("xdg-open")
        .arg(video_path)
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn play_video(video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // macOS: Use open command
    Command::new("open")
        .arg(video_path)
        .spawn()?;
    Ok(())
}

/// Show file in file explorer
#[cfg(target_os = "windows")]
pub fn show_in_folder(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("explorer")
        .args(&["/select,", file_path.to_str().unwrap()])
        .spawn()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn show_in_folder(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = file_path.parent() {
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open").arg(parent).spawn()?;
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(parent).spawn()?;
        }
    }
    Ok(())
}
