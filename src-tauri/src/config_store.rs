use std::fs;
use std::path::PathBuf;

use tauri::Manager;

use crate::models::AppConfig;

/// Resolve the config file path inside Tauri's app data directory.
pub fn config_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let base = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("tunnelhub_config.json")
}

/// Load the app configuration from disk. Returns default if file missing.
pub fn load_config(app_handle: &tauri::AppHandle) -> AppConfig {
    let path = config_path(app_handle);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                log::warn!("Failed to read config file: {}", e);
                AppConfig::default()
            }
        }
    } else {
        AppConfig::default()
    }
}

/// Save the app configuration to disk.
pub fn save_config(app_handle: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app_handle);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create config dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

/// Export config to an arbitrary file path.
pub fn export_config(path: &str, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(path, json).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

/// Import config from an arbitrary file path.
pub fn import_config(path: &str) -> Result<AppConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Read error: {e}"))?;
    let config: AppConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Parse error: {e}"))?;
    Ok(config)
}
