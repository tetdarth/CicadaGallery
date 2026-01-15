use crate::models::{VideoDatabase, VideoFile, SceneInfo, AppSettings};
use std::path::PathBuf;
use std::fs;
use std::cell::RefCell;
use rusqlite::{Connection, params, Result as SqlResult};
use chrono::{DateTime, Utc};

// Thread-local storage for current profile name
thread_local! {
    static CURRENT_PROFILE: RefCell<String> = RefCell::new("default".to_string());
}

/// Set the current profile for database operations
pub fn set_current_profile(profile_name: &str) {
    CURRENT_PROFILE.with(|p| {
        *p.borrow_mut() = profile_name.to_string();
    });
}

/// Get the current profile name
pub fn get_current_profile() -> String {
    CURRENT_PROFILE.with(|p| p.borrow().clone())
}

/// Get database directory path
fn get_database_dir() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("CicadaGallery");
    
    // Create directory if it doesn't exist
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    
    path
}

/// Get SQLite database file path
pub fn get_database_path() -> PathBuf {
    let mut path = get_database_dir();
    path.push("database.db");
    path
}

/// Get legacy JSON database file path (for migration)
pub fn get_legacy_database_path() -> PathBuf {
    let mut path = get_database_dir();
    path.push("database.json");
    path
}

/// Get settings file path (still using JSON for simplicity)
pub fn get_settings_path() -> PathBuf {
    let mut path = get_database_dir();
    path.push("settings.json");
    path
}

/// Open or create the SQLite database connection for the current profile
pub fn open_connection() -> SqlResult<Connection> {
    let profile = get_current_profile();
    let path = if profile == "default" {
        get_database_path()
    } else {
        get_profile_database_path(&profile)
    };
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    
    let conn = Connection::open(&path)?;
    
    // Enable WAL mode for better performance
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    
    Ok(conn)
}

/// Initialize the database schema
pub fn init_database(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS videos (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            duration REAL,
            file_size INTEGER NOT NULL,
            resolution_width INTEGER,
            resolution_height INTEGER,
            thumbnail_path TEXT,
            folder TEXT,
            rating INTEGER NOT NULL DEFAULT 0,
            added_date TEXT NOT NULL,
            last_played TEXT
        );
        
        CREATE TABLE IF NOT EXISTS video_tags (
            video_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (video_id, tag),
            FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
        );
        
        CREATE TABLE IF NOT EXISTS scenes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id TEXT NOT NULL,
            timestamp REAL NOT NULL,
            thumbnail_path TEXT NOT NULL,
            FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
        );
        
        CREATE TABLE IF NOT EXISTS folders (
            name TEXT PRIMARY KEY
        );
        
        CREATE TABLE IF NOT EXISTS tags (
            name TEXT PRIMARY KEY
        );
        
        CREATE INDEX IF NOT EXISTS idx_video_path ON videos(path);
        CREATE INDEX IF NOT EXISTS idx_video_folder ON videos(folder);
        CREATE INDEX IF NOT EXISTS idx_video_rating ON videos(rating);
        CREATE INDEX IF NOT EXISTS idx_video_tags_tag ON video_tags(tag);
        CREATE INDEX IF NOT EXISTS idx_scenes_video ON scenes(video_id);
        "
    )?;
    
    Ok(())
}

/// Migrate from legacy JSON database if it exists
pub fn migrate_from_json(conn: &Connection) -> Result<bool, Box<dyn std::error::Error>> {
    let legacy_path = get_legacy_database_path();
    
    if !legacy_path.exists() {
        return Ok(false);
    }
    
    eprintln!("[Migration] Found legacy JSON database, migrating to SQLite...");
    
    let json = fs::read_to_string(&legacy_path)?;
    let database: VideoDatabase = serde_json::from_str(&json)?;
    
    // Migrate folders
    for folder in &database.folders {
        conn.execute("INSERT OR IGNORE INTO folders (name) VALUES (?1)", params![folder])?;
    }
    
    // Migrate tags
    for tag in &database.tags {
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![tag])?;
    }
    
    // Migrate videos
    for video in &database.videos {
        insert_video(conn, video)?;
    }
    
    // Rename legacy file to .bak
    let backup_path = legacy_path.with_extension("json.bak");
    fs::rename(&legacy_path, &backup_path)?;
    
    eprintln!("[Migration] Successfully migrated {} videos to SQLite", database.videos.len());
    
    Ok(true)
}

/// Insert a video into the database
pub fn insert_video(conn: &Connection, video: &VideoFile) -> SqlResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO videos (id, path, title, duration, file_size, resolution_width, resolution_height, thumbnail_path, folder, rating, added_date, last_played)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            video.id,
            video.path.to_string_lossy(),
            video.title,
            video.duration,
            video.file_size as i64,
            video.resolution.map(|r| r.0 as i64),
            video.resolution.map(|r| r.1 as i64),
            video.thumbnail_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            video.folder,
            video.rating as i64,
            video.added_date.to_rfc3339(),
            video.last_played.map(|d| d.to_rfc3339()),
        ],
    )?;
    
    // Insert tags
    conn.execute("DELETE FROM video_tags WHERE video_id = ?1", params![video.id])?;
    for tag in &video.tags {
        conn.execute(
            "INSERT OR IGNORE INTO video_tags (video_id, tag) VALUES (?1, ?2)",
            params![video.id, tag],
        )?;
    }
    
    // Insert scenes
    conn.execute("DELETE FROM scenes WHERE video_id = ?1", params![video.id])?;
    for scene in &video.scenes {
        conn.execute(
            "INSERT INTO scenes (video_id, timestamp, thumbnail_path) VALUES (?1, ?2, ?3)",
            params![
                video.id,
                scene.timestamp,
                scene.thumbnail_path.to_string_lossy(),
            ],
        )?;
    }
    
    // Ensure folder is in folders table
    if let Some(ref folder) = video.folder {
        conn.execute("INSERT OR IGNORE INTO folders (name) VALUES (?1)", params![folder])?;
    }
    
    Ok(())
}

/// Update a video in the database
pub fn update_video(conn: &Connection, video: &VideoFile) -> SqlResult<()> {
    insert_video(conn, video)
}

/// Delete a video from the database
pub fn delete_video(conn: &Connection, video_id: &str) -> SqlResult<()> {
    conn.execute("DELETE FROM video_tags WHERE video_id = ?1", params![video_id])?;
    conn.execute("DELETE FROM scenes WHERE video_id = ?1", params![video_id])?;
    conn.execute("DELETE FROM videos WHERE id = ?1", params![video_id])?;
    Ok(())
}

/// Load a single video from a row
fn video_from_row(row: &rusqlite::Row) -> SqlResult<VideoFile> {
    let id: String = row.get(0)?;
    let path_str: String = row.get(1)?;
    let title: String = row.get(2)?;
    let duration: Option<f64> = row.get(3)?;
    let file_size: i64 = row.get(4)?;
    let resolution_width: Option<i64> = row.get(5)?;
    let resolution_height: Option<i64> = row.get(6)?;
    let thumbnail_path_str: Option<String> = row.get(7)?;
    let folder: Option<String> = row.get(8)?;
    let rating: i64 = row.get(9)?;
    let added_date_str: String = row.get(10)?;
    let last_played_str: Option<String> = row.get(11)?;
    
    let resolution = match (resolution_width, resolution_height) {
        (Some(w), Some(h)) => Some((w as u32, h as u32)),
        _ => None,
    };
    
    let thumbnail_path = thumbnail_path_str.map(PathBuf::from);
    
    let added_date = DateTime::parse_from_rfc3339(&added_date_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    
    let last_played = last_played_str.and_then(|s| {
        DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))
    });
    
    Ok(VideoFile {
        id,
        path: PathBuf::from(path_str),
        title,
        duration,
        file_size: file_size as u64,
        resolution,
        frame_rate: None, // Frame rate is loaded separately if needed
        thumbnail_path,
        tags: Vec::new(), // Will be filled separately
        folder,
        rating: rating as u8,
        is_favorite_legacy: None,
        added_date,
        last_played,
        scenes: Vec::new(), // Will be filled separately
    })
}

/// Load tags for a video
fn load_video_tags(conn: &Connection, video_id: &str) -> SqlResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT tag FROM video_tags WHERE video_id = ?1")?;
    let tags = stmt.query_map(params![video_id], |row| row.get(0))?
        .collect::<SqlResult<Vec<String>>>()?;
    Ok(tags)
}

/// Load scenes for a video
fn load_video_scenes(conn: &Connection, video_id: &str) -> SqlResult<Vec<SceneInfo>> {
    let mut stmt = conn.prepare("SELECT timestamp, thumbnail_path FROM scenes WHERE video_id = ?1 ORDER BY timestamp")?;
    let scenes = stmt.query_map(params![video_id], |row| {
        let timestamp: f64 = row.get(0)?;
        let path_str: String = row.get(1)?;
        Ok(SceneInfo {
            timestamp,
            thumbnail_path: PathBuf::from(path_str),
        })
    })?.collect::<SqlResult<Vec<SceneInfo>>>()?;
    Ok(scenes)
}

/// Load a video by ID with all related data
pub fn load_video_by_id(conn: &Connection, video_id: &str) -> SqlResult<Option<VideoFile>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, title, duration, file_size, resolution_width, resolution_height, 
                thumbnail_path, folder, rating, added_date, last_played 
         FROM videos WHERE id = ?1"
    )?;
    
    let mut rows = stmt.query(params![video_id])?;
    
    if let Some(row) = rows.next()? {
        let mut video = video_from_row(row)?;
        video.tags = load_video_tags(conn, &video.id)?;
        video.scenes = load_video_scenes(conn, &video.id)?;
        Ok(Some(video))
    } else {
        Ok(None)
    }
}

/// Check if a video path exists in the database
pub fn has_video_path(conn: &Connection, path: &PathBuf) -> SqlResult<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM videos WHERE path = ?1 LIMIT 1")?;
    let exists = stmt.exists(params![path.to_string_lossy()])?;
    Ok(exists)
}

/// Get video by path
pub fn get_video_by_path(conn: &Connection, path: &PathBuf) -> SqlResult<Option<VideoFile>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, title, duration, file_size, resolution_width, resolution_height, 
                thumbnail_path, folder, rating, added_date, last_played 
         FROM videos WHERE path = ?1"
    )?;
    
    let path_str = path.to_string_lossy();
    let mut rows = stmt.query(params![path_str])?;
    
    if let Some(row) = rows.next()? {
        let mut video = video_from_row(row)?;
        video.tags = load_video_tags(conn, &video.id)?;
        video.scenes = load_video_scenes(conn, &video.id)?;
        Ok(Some(video))
    } else {
        Ok(None)
    }
}

/// Load all videos from the database (optimized with batch loading)
pub fn load_all_videos(conn: &Connection) -> SqlResult<Vec<VideoFile>> {
    // Load all videos
    let mut stmt = conn.prepare(
        "SELECT id, path, title, duration, file_size, resolution_width, resolution_height, 
                thumbnail_path, folder, rating, added_date, last_played 
         FROM videos"
    )?;
    
    let video_rows = stmt.query_map([], video_from_row)?
        .collect::<SqlResult<Vec<VideoFile>>>()?;
    
    // Build a map of video_id -> video for efficient lookup
    let mut video_map: std::collections::HashMap<String, VideoFile> = 
        video_rows.into_iter().map(|v| (v.id.clone(), v)).collect();
    
    // Batch load all tags
    let mut tag_stmt = conn.prepare("SELECT video_id, tag FROM video_tags ORDER BY video_id")?;
    let tag_rows = tag_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    
    for tag_result in tag_rows {
        if let Ok((video_id, tag)) = tag_result {
            if let Some(video) = video_map.get_mut(&video_id) {
                video.tags.push(tag);
            }
        }
    }
    
    // Batch load all scenes
    let mut scene_stmt = conn.prepare(
        "SELECT video_id, timestamp, thumbnail_path FROM scenes ORDER BY video_id, timestamp"
    )?;
    let scene_rows = scene_stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, String>(2)?
        ))
    })?;
    
    for scene_result in scene_rows {
        if let Ok((video_id, timestamp, path_str)) = scene_result {
            if let Some(video) = video_map.get_mut(&video_id) {
                video.scenes.push(SceneInfo {
                    timestamp,
                    thumbnail_path: PathBuf::from(path_str),
                });
            }
        }
    }
    
    // Convert map back to vector
    Ok(video_map.into_values().collect())
}

/// Load all folders from the database
pub fn load_all_folders(conn: &Connection) -> SqlResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM folders ORDER BY name")?;
    let folders = stmt.query_map([], |row| row.get(0))?
        .collect::<SqlResult<Vec<String>>>()?;
    Ok(folders)
}

/// Load all tags from the database
pub fn load_all_tags(conn: &Connection) -> SqlResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name")?;
    let tags = stmt.query_map([], |row| row.get(0))?
        .collect::<SqlResult<Vec<String>>>()?;
    Ok(tags)
}

/// Add a folder
pub fn add_folder(conn: &Connection, folder: &str) -> SqlResult<()> {
    conn.execute("INSERT OR IGNORE INTO folders (name) VALUES (?1)", params![folder])?;
    Ok(())
}

/// Add a tag
pub fn add_tag(conn: &Connection, tag: &str) -> SqlResult<()> {
    conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![tag])?;
    Ok(())
}

/// Remove a folder
pub fn remove_folder(conn: &Connection, folder: &str) -> SqlResult<()> {
    conn.execute("DELETE FROM folders WHERE name = ?1", params![folder])?;
    Ok(())
}

/// Remove a tag
pub fn remove_tag(conn: &Connection, tag: &str) -> SqlResult<()> {
    conn.execute("DELETE FROM tags WHERE name = ?1", params![tag])?;
    Ok(())
}

/// Remove unused folders (not associated with any video)
pub fn cleanup_unused_folders(conn: &Connection) -> SqlResult<usize> {
    let result = conn.execute(
        "DELETE FROM folders WHERE name NOT IN (SELECT DISTINCT folder FROM videos WHERE folder IS NOT NULL)",
        [],
    )?;
    Ok(result)
}

/// Remove unused tags (not associated with any video)
pub fn cleanup_unused_tags(conn: &Connection) -> SqlResult<usize> {
    let result = conn.execute(
        "DELETE FROM tags WHERE name NOT IN (SELECT DISTINCT tag FROM video_tags)",
        [],
    )?;
    Ok(result)
}

/// Get count of videos
pub fn get_video_count(conn: &Connection) -> SqlResult<usize> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM videos", [], |row| row.get(0))?;
    Ok(count as usize)
}

/// Remove duplicate videos (keep first occurrence)
pub fn remove_duplicates(conn: &Connection) -> SqlResult<usize> {
    // SQLite handles duplicates via UNIQUE constraint on path, so this is mainly for cleanup
    let result = conn.execute(
        "DELETE FROM videos WHERE rowid NOT IN (SELECT MIN(rowid) FROM videos GROUP BY path)",
        [],
    )?;
    Ok(result)
}

/// Update added_date for all videos from file metadata
pub fn update_added_dates_from_files(conn: &Connection) -> SqlResult<usize> {
    use std::time::UNIX_EPOCH;
    
    let videos = load_all_videos(conn)?;
    let mut updated = 0;
    
    for video in videos {
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
                        conn.execute(
                            "UPDATE videos SET added_date = ?1 WHERE id = ?2",
                            params![datetime.to_rfc3339(), video.id],
                        )?;
                        updated += 1;
                    }
                }
            }
        }
    }
    
    Ok(updated)
}

// ============================================================================
// High-level API (compatible with existing code)
// ============================================================================

/// Database wrapper for easier use throughout the application
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = open_connection()?;
        init_database(&conn)?;
        
        // Try to migrate from JSON if exists
        let _ = migrate_from_json(&conn);
        
        Ok(Self { conn })
    }
    
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

/// Save database (compatibility wrapper - saves a VideoDatabase to SQLite)
pub fn save_database(database: &VideoDatabase) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_connection()?;
    init_database(&conn)?;
    
    // Use a transaction for better performance
    conn.execute("BEGIN TRANSACTION", [])?;
    
    // Clear and rebuild
    conn.execute("DELETE FROM video_tags", [])?;
    conn.execute("DELETE FROM scenes", [])?;
    conn.execute("DELETE FROM videos", [])?;
    conn.execute("DELETE FROM folders", [])?;
    conn.execute("DELETE FROM tags", [])?;
    
    for folder in &database.folders {
        add_folder(&conn, folder)?;
    }
    
    for tag in &database.tags {
        add_tag(&conn, tag)?;
    }
    
    for video in &database.videos {
        insert_video(&conn, video)?;
    }
    
    conn.execute("COMMIT", [])?;
    
    Ok(())
}

/// Load database (compatibility wrapper - loads VideoDatabase from SQLite)
pub fn load_database() -> Result<VideoDatabase, Box<dyn std::error::Error>> {
    let conn = open_connection()?;
    init_database(&conn)?;
    
    // Try to migrate from JSON if exists
    let _ = migrate_from_json(&conn);
    
    let videos = load_all_videos(&conn)?;
    let folders = load_all_folders(&conn)?;
    let tags = load_all_tags(&conn)?;
    
    Ok(VideoDatabase { videos, folders, tags })
}

/// Save settings to file (still using JSON)
pub fn save_settings(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_settings_path();
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load settings from file (still using JSON)
pub fn load_settings() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let path = get_settings_path();
    
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    
    let json = fs::read_to_string(path)?;
    let settings = serde_json::from_str(&json)?;
    Ok(settings)
}

/// Get backup directory path
pub fn get_backup_dir() -> PathBuf {
    let mut path = get_database_dir();
    path.push("backups");
    
    // Create directory if it doesn't exist
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    
    path
}

/// Get backup database file path with timestamp
pub fn get_backup_path_with_timestamp(timestamp: &str) -> PathBuf {
    let mut path = get_backup_dir();
    path.push(format!("database_backup_{}.db", timestamp));
    path
}

/// Get list of all backup files sorted by date (newest first)
pub fn list_backups() -> Result<Vec<(PathBuf, String)>, Box<dyn std::error::Error>> {
    let backup_dir = get_backup_dir();
    let mut backups: Vec<(PathBuf, String)> = Vec::new();
    
    if backup_dir.exists() {
        for entry in fs::read_dir(&backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "db" {
                    if let Some(filename) = path.file_stem() {
                        let name = filename.to_string_lossy();
                        if name.starts_with("database_backup_") {
                            // Extract timestamp from filename
                            let timestamp = name.trim_start_matches("database_backup_").to_string();
                            backups.push((path, timestamp));
                        }
                    }
                }
            }
        }
    }
    
    // Sort by timestamp (newest first)
    backups.sort_by(|a, b| b.1.cmp(&a.1));
    
    Ok(backups)
}

/// Remove old backups, keeping only the latest N
fn cleanup_old_backups(keep_count: usize) -> Result<usize, Box<dyn std::error::Error>> {
    let backups = list_backups()?;
    let mut removed = 0;
    
    if backups.len() > keep_count {
        for (path, _) in backups.into_iter().skip(keep_count) {
            if fs::remove_file(&path).is_ok() {
                eprintln!("[Backup] Removed old backup: {:?}", path);
                removed += 1;
            }
        }
    }
    
    Ok(removed)
}

/// Check if backup is needed (every 3 days)
pub fn should_backup(settings: &AppSettings) -> bool {
    if let Some(ref last_backup_str) = settings.last_backup_date {
        if let Ok(last_backup) = DateTime::parse_from_rfc3339(last_backup_str) {
            let now = Utc::now();
            let last_backup_utc = last_backup.with_timezone(&Utc);
            let days_since_backup = (now - last_backup_utc).num_days();
            return days_since_backup >= 3;
        }
    }
    // No backup date recorded, should backup
    true
}

/// Create a backup of the database with timestamp filename
pub fn create_backup() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source_path = get_database_path();
    
    if source_path.exists() {
        // Create timestamp for filename (format: YYYYMMDD_HHMMSS)
        let now = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        
        let backup_path = get_backup_path_with_timestamp(&timestamp);
        let conn = open_connection()?;
        let backup_path_str = backup_path.to_string_lossy();
        
        conn.execute(&format!("VACUUM INTO '{}'", backup_path_str), [])?;
        eprintln!("[Backup] Database backed up to {:?}", backup_path);
        
        // Cleanup old backups (keep only 3)
        let _ = cleanup_old_backups(3);
        
        return Ok(backup_path);
    }
    
    Err("Source database does not exist".into())
}

/// Restore database from a backup file
pub fn restore_from_backup(backup_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !backup_path.exists() {
        return Err("Backup file does not exist".into());
    }
    
    let db_path = get_database_path();
    
    // Create a safety backup of current database before restore
    if db_path.exists() {
        let now = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let safety_backup = get_backup_path_with_timestamp(&format!("{}_before_restore", timestamp));
        
        // Close any existing connections by opening a new one and using VACUUM INTO
        let conn = open_connection()?;
        conn.execute(&format!("VACUUM INTO '{}'", safety_backup.to_string_lossy()), [])?;
        eprintln!("[Restore] Safety backup created at {:?}", safety_backup);
    }
    
    // Copy backup to database location
    fs::copy(backup_path, &db_path)?;
    eprintln!("[Restore] Database restored from {:?}", backup_path);
    
    Ok(())
}

/// Get formatted display name for backup timestamp
pub fn format_backup_timestamp(timestamp: &str) -> String {
    // Parse YYYYMMDD_HHMMSS format
    if timestamp.len() >= 15 {
        let year = &timestamp[0..4];
        let month = &timestamp[4..6];
        let day = &timestamp[6..8];
        let hour = &timestamp[9..11];
        let min = &timestamp[11..13];
        let sec = &timestamp[13..15];
        format!("{}/{}/{} {}:{}:{}", year, month, day, hour, min, sec)
    } else {
        timestamp.to_string()
    }
}

/// Perform backup if needed and update settings
pub fn perform_backup_if_needed(settings: &mut AppSettings) -> Result<bool, Box<dyn std::error::Error>> {
    if should_backup(settings) {
        create_backup()?;
        settings.last_backup_date = Some(Utc::now().to_rfc3339());
        save_settings(settings)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Optimize the database (VACUUM and ANALYZE)
pub fn optimize_database() -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_connection()?;
    conn.execute_batch("VACUUM; ANALYZE;")?;
    eprintln!("[Optimize] Database optimized (VACUUM and ANALYZE)");
    Ok(())
}

// ============================================================================
// Profile Management
// ============================================================================

/// Get profiles directory path
pub fn get_profiles_dir() -> PathBuf {
    let mut path = get_database_dir();
    path.push("profiles");
    
    // Create directory if it doesn't exist
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    
    path
}

/// Get profile directory path for a specific profile (does NOT auto-create)
fn get_profile_dir_path(profile_name: &str) -> PathBuf {
    let mut path = get_profiles_dir();
    path.push(profile_name);
    path
}

/// Get profile directory path for a specific profile (auto-creates if needed)
pub fn get_profile_dir(profile_name: &str) -> PathBuf {
    let path = get_profile_dir_path(profile_name);
    
    // Create directory if it doesn't exist
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    
    path
}

/// Get database path for a specific profile
pub fn get_profile_database_path(profile_name: &str) -> PathBuf {
    let mut path = get_profile_dir(profile_name);
    path.push("database.db");
    path
}

/// Get cache directory for a specific profile
pub fn get_profile_cache_dir(profile_name: &str) -> PathBuf {
    let mut path = get_profile_dir(profile_name);
    path.push("cache");
    
    // Create directory if it doesn't exist
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    
    path
}

/// List all available profiles
pub fn list_profiles() -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let profiles_dir = get_profiles_dir();
    let mut profiles = Vec::new();
    
    // Add "default" profile (from the original database location)
    let default_db = get_database_path();
    let default_count = if default_db.exists() {
        // Count videos in default database
        match Connection::open(&default_db) {
            Ok(conn) => {
                conn.query_row("SELECT COUNT(*) FROM videos", [], |row| row.get(0)).unwrap_or(0)
            }
            Err(_) => 0
        }
    } else {
        0
    };
    profiles.push(("default".to_string(), default_count));
    
    // Scan profiles directory
    if profiles_dir.exists() {
        if let Ok(entries) = fs::read_dir(&profiles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let profile_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());
                    
                    if let Some(name) = profile_name {
                        // Skip if it's "default" (already added)
                        if name == "default" {
                            continue;
                        }
                        
                        // Count videos in this profile's database
                        let db_path = get_profile_database_path(&name);
                        let count = if db_path.exists() {
                            match Connection::open(&db_path) {
                                Ok(conn) => {
                                    conn.query_row("SELECT COUNT(*) FROM videos", [], |row| row.get(0)).unwrap_or(0)
                                }
                                Err(_) => 0
                            }
                        } else {
                            0
                        };
                        
                        profiles.push((name, count));
                    }
                }
            }
        }
    }
    
    // Sort by name (but keep "default" first)
    profiles.sort_by(|a, b| {
        if a.0 == "default" {
            std::cmp::Ordering::Less
        } else if b.0 == "default" {
            std::cmp::Ordering::Greater
        } else {
            a.0.cmp(&b.0)
        }
    });
    
    Ok(profiles)
}

/// Create a new profile
pub fn create_profile(profile_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Validate profile name
    if profile_name.is_empty() || profile_name == "default" {
        return Err("Invalid profile name".into());
    }
    
    // Check for invalid characters
    if profile_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Profile name contains invalid characters".into());
    }
    
    // Use non-auto-creating path check
    let profile_dir = get_profile_dir_path(profile_name);
    
    // Check if profile already exists
    if profile_dir.exists() {
        return Err("Profile already exists".into());
    }
    
    // Create profile directory
    fs::create_dir_all(&profile_dir)?;
    
    // Create cache directory
    let mut cache_dir = profile_dir.clone();
    cache_dir.push("cache");
    fs::create_dir_all(&cache_dir)?;
    
    // Initialize empty database for this profile
    let mut db_path = profile_dir;
    db_path.push("database.db");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    init_database(&conn)?;
    
    eprintln!("[Profile] Created new profile: {}", profile_name);
    Ok(())
}

/// Delete a profile
pub fn delete_profile(profile_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Cannot delete default profile
    if profile_name == "default" {
        return Err("Cannot delete the default profile".into());
    }
    
    let profile_dir = get_profile_dir_path(profile_name);
    
    if !profile_dir.exists() {
        return Err("Profile does not exist".into());
    }
    
    // Delete the entire profile directory
    fs::remove_dir_all(&profile_dir)?;
    
    eprintln!("[Profile] Deleted profile: {}", profile_name);
    Ok(())
}

/// Rename a profile
pub fn rename_profile(old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Cannot rename default profile
    if old_name == "default" {
        return Err("Cannot rename the default profile".into());
    }
    
    // Validate new name
    if new_name.is_empty() || new_name == "default" {
        return Err("Invalid new profile name".into());
    }
    
    if new_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("New profile name contains invalid characters".into());
    }
    
    let old_dir = get_profile_dir(old_name);
    let new_dir = get_profiles_dir().join(new_name);
    
    if !old_dir.exists() {
        return Err("Profile does not exist".into());
    }
    
    if new_dir.exists() {
        return Err("A profile with the new name already exists".into());
    }
    
    fs::rename(&old_dir, &new_dir)?;
    
    eprintln!("[Profile] Renamed profile: {} -> {}", old_name, new_name);
    Ok(())
}

/// Open connection to a specific profile's database
pub fn open_profile_connection(profile_name: &str) -> SqlResult<Connection> {
    let path = if profile_name == "default" {
        get_database_path()
    } else {
        get_profile_database_path(profile_name)
    };
    
    let conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    
    Ok(conn)
}

/// Load database for a specific profile
pub fn load_profile_database(profile_name: &str) -> Result<VideoDatabase, Box<dyn std::error::Error>> {
    let db_path = if profile_name == "default" {
        get_database_path()
    } else {
        get_profile_database_path(profile_name)
    };
    
    // If database doesn't exist, return empty database
    if !db_path.exists() {
        return Ok(VideoDatabase::new());
    }
    
    let conn = open_profile_connection(profile_name)?;
    init_database(&conn)?;
    
    // Load from SQLite (use the same loading logic as load_database)
    let videos = load_all_videos(&conn)?;
    let folders = load_all_folders(&conn)?;
    let tags = load_all_tags(&conn)?;
    
    Ok(VideoDatabase { videos, folders, tags })
}

/// Save database for a specific profile
pub fn save_profile_database(profile_name: &str, database: &VideoDatabase) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_profile_connection(profile_name)?;
    init_database(&conn)?;
    save_database(&database)?;
    Ok(())
}

/// Get thumbnail directory for a specific profile
pub fn get_profile_thumbnail_dir(profile_name: &str) -> PathBuf {
    let cache_dir = if profile_name == "default" {
        let mut path = get_database_dir();
        path.push("cache");
        path
    } else {
        get_profile_cache_dir(profile_name)
    };
    
    let mut thumb_dir = cache_dir;
    thumb_dir.push("thumbnails");
    
    if !thumb_dir.exists() {
        let _ = fs::create_dir_all(&thumb_dir);
    }
    
    thumb_dir
}

/// Get scenes directory for a specific profile
pub fn get_profile_scenes_dir(profile_name: &str) -> PathBuf {
    let cache_dir = if profile_name == "default" {
        let mut path = get_database_dir();
        path.push("cache");
        path
    } else {
        get_profile_cache_dir(profile_name)
    };
    
    let mut scenes_dir = cache_dir;
    scenes_dir.push("scenes");
    
    if !scenes_dir.exists() {
        let _ = fs::create_dir_all(&scenes_dir);
    }
    
    scenes_dir
}
