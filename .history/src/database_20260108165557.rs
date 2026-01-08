use crate::models::{VideoDatabase, AppSettings};
use std::path::PathBuf;
use std::fs;

/// Get database file path
pub fn get_database_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("cicadaGallaley");
    
    // Create directory if it doesn't exist
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    
    path.push("database.json");
    path
}

/// Get settings file path
pub fn get_settings_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("cicadaGallaley");
    
    // Create directory if it doesn't exist
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    
    path.push("settings.json");
    path
}

/// Save database to file
pub fn save_database(database: &VideoDatabase) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_database_path();
    let json = serde_json::to_string_pretty(database)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load database from file
pub fn load_database() -> Result<VideoDatabase, Box<dyn std::error::Error>> {
    let path = get_database_path();
    
    if !path.exists() {
        return Ok(VideoDatabase::new());
    }
    
    let json = fs::read_to_string(path)?;
    let database = serde_json::from_str(&json)?;
    Ok(database)
}

/// Save settings to file
pub fn save_settings(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_settings_path();
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load settings from file
pub fn load_settings() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let path = get_settings_path();
    
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    
    let json = fs::read_to_string(path)?;
    let settings = serde_json::from_str(&json)?;
    Ok(settings)
}
