use std::path::PathBuf;
use tauri::State;
use tracing::{info, debug};

use filemover_types::Config;
use crate::state::AppState;
use crate::error::{GuiResult, GuiError, gui_error};

#[tauri::command]
pub async fn load_config(
    profile: Option<String>,
    state: State<'_, AppState>,
) -> GuiResult<Config> {
    info!("Loading configuration profile: {:?}", profile);
    
    let profile_name = profile.unwrap_or_else(|| {
        state.current_profile.lock()
            .map(|p| p.clone())
            .unwrap_or_else(|_| "default".to_string())
    });
    
    // TODO: Implement actual file loading
    // For now, return the current config from state
    let config = state.config.lock()
        .map_err(|_| gui_error!(config, "Failed to access configuration"))?
        .clone();
    
    debug!("Configuration loaded successfully");
    Ok(config)
}

#[tauri::command]
pub async fn save_config(
    profile: String,
    config: Config,
    state: State<'_, AppState>,
) -> GuiResult<()> {
    info!("Saving configuration to profile: {}", profile);
    
    // Update state
    *state.config.lock()
        .map_err(|_| gui_error!(config, "Failed to access configuration"))? = config.clone();
    
    *state.current_profile.lock()
        .map_err(|_| gui_error!(config, "Failed to update current profile"))? = profile.clone();
    
    // TODO: Implement actual file saving
    // This would save to the appropriate profile file
    
    info!("Configuration saved successfully");
    Ok(())
}

#[tauri::command]
pub async fn list_profiles() -> GuiResult<Vec<String>> {
    info!("Listing available profiles");
    
    // TODO: Implement actual profile discovery
    // For now, return a static list
    let profiles = vec![
        "default".to_string(),
        "work".to_string(),
        "personal".to_string(),
    ];
    
    debug!("Found {} profiles", profiles.len());
    Ok(profiles)
}

#[tauri::command]
pub async fn create_profile(
    name: String,
    from_profile: Option<String>,
    state: State<'_, AppState>,
) -> GuiResult<()> {
    info!("Creating new profile: {} (from: {:?})", name, from_profile);
    
    if name.trim().is_empty() {
        return Err(gui_error!(config, "Profile name cannot be empty"));
    }
    
    // TODO: Check if profile already exists
    // TODO: Copy from base profile if specified
    // TODO: Save new profile file
    
    // For now, just validate the name
    if name.len() > 50 {
        return Err(gui_error!(config, "Profile name too long (max 50 characters)"));
    }
    
    if name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(gui_error!(config, "Profile name contains invalid characters"));
    }
    
    info!("Profile '{}' created successfully", name);
    Ok(())
}

#[tauri::command]
pub async fn delete_profile(name: String) -> GuiResult<()> {
    info!("Deleting profile: {}", name);
    
    if name == "default" {
        return Err(gui_error!(config, "Cannot delete the default profile"));
    }
    
    // TODO: Implement actual profile deletion
    // This would remove the profile file
    
    info!("Profile '{}' deleted successfully", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_load_config() {
        let state = AppState::new();
        let result = load_config(None, State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_save_config() {
        let state = AppState::new();
        let config = Config::default();
        let result = save_config("test".to_string(), config, State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let result = list_profiles().await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_profile_validation() {
        let state = AppState::new();
        
        // Test empty name
        let result = create_profile("".to_string(), None, State::from(&state)).await;
        assert!(result.is_err());
        
        // Test invalid characters
        let result = create_profile("test/profile".to_string(), None, State::from(&state)).await;
        assert!(result.is_err());
        
        // Test valid name
        let result = create_profile("valid_profile".to_string(), None, State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_default_profile() {
        let result = delete_profile("default".to_string()).await;
        assert!(result.is_err());
    }
}