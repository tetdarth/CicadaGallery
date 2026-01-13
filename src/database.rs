use crate::models::{VideoDatabase, VideoFile, SceneInfo, AppSettings};
use std::path::PathBuf;
use std::fs;
use rusqlite::{Connection, params, Result as SqlResult};
use chrono::{DateTime, Utc};

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

/// Open or create the SQLite database connection
pub fn open_connection() -> SqlResult<Connection> {
    let path = get_database_path();
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

/// Get backup database file path (with generation number)
pub fn get_backup_path_with_generation(generation: u8) -> PathBuf {
    let mut path = get_database_dir();
    path.push(format!("database_backup_{}.db", generation));
    path
}

/// Get backup database file path (latest backup = generation 1)
pub fn get_backup_path() -> PathBuf {
    get_backup_path_with_generation(1)
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

/// Rotate backup files (keep up to 3 generations)
/// Generation 1 is the newest, generation 3 is the oldest
fn rotate_backups() -> Result<(), Box<dyn std::error::Error>> {
    // Delete generation 3 (oldest) if exists
    let backup_3 = get_backup_path_with_generation(3);
    if backup_3.exists() {
        fs::remove_file(&backup_3)?;
        eprintln!("[Backup] Removed oldest backup (generation 3)");
    }
    
    // Rename generation 2 -> 3
    let backup_2 = get_backup_path_with_generation(2);
    if backup_2.exists() {
        fs::rename(&backup_2, &backup_3)?;
        eprintln!("[Backup] Rotated generation 2 -> 3");
    }
    
    // Rename generation 1 -> 2
    let backup_1 = get_backup_path_with_generation(1);
    if backup_1.exists() {
        fs::rename(&backup_1, &backup_2)?;
        eprintln!("[Backup] Rotated generation 1 -> 2");
    }
    
    Ok(())
}

/// Create a backup of the database (with 3-generation rotation)
pub fn create_backup() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source_path = get_database_path();
    
    if source_path.exists() {
        // Rotate existing backups first
        rotate_backups()?;
        
        // Create new backup as generation 1
        let backup_path = get_backup_path_with_generation(1);
        let conn = open_connection()?;
        let backup_path_str = backup_path.to_string_lossy();
        
        conn.execute(&format!("VACUUM INTO '{}'", backup_path_str), [])?;
        eprintln!("[Backup] Database backed up to {:?}", backup_path);
        
        return Ok(backup_path);
    }
    
    Ok(get_backup_path())
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
