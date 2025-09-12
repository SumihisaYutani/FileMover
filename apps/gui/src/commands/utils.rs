use std::path::PathBuf;
use tauri::api::dialog;
use serde::{Deserialize, Serialize};

use crate::error::{GuiResult, GuiError, gui_error};

#[tauri::command]
pub async fn browse_folder(
    title: Option<String>,
    default_path: Option<PathBuf>,
) -> GuiResult<Option<PathBuf>> {
    let dialog_title = title.unwrap_or_else(|| "Select Folder".to_string());
    
    // This would use tauri's dialog API to show a folder picker
    // For now, returning a placeholder
    Ok(default_path)
}

#[tauri::command]
pub async fn validate_path(path: PathBuf) -> GuiResult<PathValidation> {
    let mut validation = PathValidation {
        is_valid: true,
        exists: path.exists(),
        is_directory: false,
        is_readable: false,
        is_writable: false,
        is_long_path: false,
        is_network_path: false,
        is_system_protected: false,
        warnings: vec![],
        errors: vec![],
    };
    
    if !validation.exists {
        validation.is_valid = false;
        validation.errors.push("Path does not exist".to_string());
        return Ok(validation);
    }
    
    validation.is_directory = path.is_dir();
    if !validation.is_directory {
        validation.warnings.push("Path is not a directory".to_string());
    }
    
    // Check if path is too long
    let path_str = path.to_string_lossy();
    validation.is_long_path = path_str.len() > 260;
    if validation.is_long_path {
        validation.warnings.push("Path is longer than 260 characters".to_string());
    }
    
    // Check if it's a network path
    validation.is_network_path = path_str.starts_with("\\\\");
    if validation.is_network_path {
        validation.warnings.push("Network paths may have slower performance".to_string());
    }
    
    // Check if it's a system protected path
    validation.is_system_protected = is_system_protected_path(&path);
    if validation.is_system_protected {
        validation.warnings.push("System protected path - access may be restricted".to_string());
    }
    
    // Test readability
    validation.is_readable = test_path_readable(&path);
    if !validation.is_readable {
        validation.errors.push("Path is not readable".to_string());
        validation.is_valid = false;
    }
    
    // Test writability (for destination paths)
    validation.is_writable = test_path_writable(&path);
    if !validation.is_writable {
        validation.warnings.push("Path may not be writable".to_string());
    }
    
    Ok(validation)
}

fn is_system_protected_path(path: &PathBuf) -> bool {
    let path_str = path.to_string_lossy().to_uppercase();
    
    path_str.starts_with("C:\\WINDOWS") ||
    path_str.starts_with("C:\\PROGRAM FILES") ||
    path_str.contains("$RECYCLE.BIN") ||
    path_str.contains("SYSTEM VOLUME INFORMATION")
}

fn test_path_readable(path: &PathBuf) -> bool {
    if !path.exists() {
        return false;
    }
    
    if path.is_dir() {
        // Try to read directory contents
        std::fs::read_dir(path).is_ok()
    } else {
        // Try to read file metadata
        std::fs::metadata(path).is_ok()
    }
}

fn test_path_writable(path: &PathBuf) -> bool {
    if !path.exists() {
        // Test if parent directory is writable
        if let Some(parent) = path.parent() {
            return test_path_writable(&parent.to_path_buf());
        }
        return false;
    }
    
    if path.is_dir() {
        // Try to create a test file
        let test_file = path.join(".filemover_write_test");
        match std::fs::write(&test_file, "") {
            Ok(_) => {
                let _ = std::fs::remove_file(&test_file);
                true
            }
            Err(_) => false,
        }
    } else {
        // For files, check if we can open for writing
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .is_ok()
    }
}

#[tauri::command]
pub async fn get_system_info() -> GuiResult<SystemInfo> {
    let info = SystemInfo {
        os_type: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        long_path_support: check_long_path_support(),
        available_drives: get_available_drives(),
    };
    
    Ok(info)
}

fn check_long_path_support() -> bool {
    // On Windows, check if long path support is enabled
    #[cfg(windows)]
    {
        // This is a simplified check - real implementation would check registry
        true
    }
    
    #[cfg(not(windows))]
    {
        true
    }
}

fn get_available_drives() -> Vec<DriveInfo> {
    let mut drives = Vec::new();
    
    #[cfg(windows)]
    {
        // Get Windows drive letters
        for letter in 'A'..='Z' {
            let drive_path = format!("{}:\\", letter);
            let path = PathBuf::from(&drive_path);
            
            if path.exists() {
                let drive_type = get_drive_type(&path);
                drives.push(DriveInfo {
                    path: path.clone(),
                    label: drive_path,
                    drive_type,
                    total_space: None,
                    free_space: None,
                });
            }
        }
    }
    
    #[cfg(not(windows))]
    {
        // For Unix-like systems, add root and common mount points
        drives.push(DriveInfo {
            path: PathBuf::from("/"),
            label: "Root".to_string(),
            drive_type: DriveType::Fixed,
            total_space: None,
            free_space: None,
        });
    }
    
    drives
}

fn get_drive_type(_path: &PathBuf) -> DriveType {
    // Simplified drive type detection
    DriveType::Fixed
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathValidation {
    pub is_valid: bool,
    pub exists: bool,
    pub is_directory: bool,
    pub is_readable: bool,
    pub is_writable: bool,
    pub is_long_path: bool,
    pub is_network_path: bool,
    pub is_system_protected: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_type: String,
    pub arch: String,
    pub long_path_support: bool,
    pub available_drives: Vec<DriveInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DriveInfo {
    pub path: PathBuf,
    pub label: String,
    pub drive_type: DriveType,
    pub total_space: Option<u64>,
    pub free_space: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DriveType {
    Fixed,
    Removable,
    Network,
    CD,
    Ram,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_validate_path_existing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();
        
        let result = validate_path(path).await;
        assert!(result.is_ok());
        
        let validation = result.unwrap();
        assert!(validation.is_valid);
        assert!(validation.exists);
        assert!(validation.is_directory);
    }

    #[tokio::test]
    async fn test_validate_path_nonexistent() {
        let path = PathBuf::from("/nonexistent/path");
        
        let result = validate_path(path).await;
        assert!(result.is_ok());
        
        let validation = result.unwrap();
        assert!(!validation.is_valid);
        assert!(!validation.exists);
    }

    #[test]
    fn test_is_system_protected_path() {
        assert!(is_system_protected_path(&PathBuf::from("C:\\Windows\\System32")));
        assert!(is_system_protected_path(&PathBuf::from("C:\\Program Files\\Test")));
        assert!(!is_system_protected_path(&PathBuf::from("C:\\Users\\Test")));
    }

    #[tokio::test]
    async fn test_get_system_info() {
        let result = get_system_info().await;
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert!(!info.os_type.is_empty());
        assert!(!info.arch.is_empty());
    }

    #[test]
    fn test_get_available_drives() {
        let drives = get_available_drives();
        assert!(!drives.is_empty());
    }
}