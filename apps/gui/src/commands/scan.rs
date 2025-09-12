use std::path::PathBuf;
use tauri::State;
use tracing::{info, debug, warn};
use uuid::Uuid;

use filemover_types::{FolderHit, ScanOptions};
use filemover_scanner::FolderScanner;
use crate::state::{AppState, SessionStatus};
use crate::error::{GuiResult, GuiError, gui_error};

#[tauri::command]
pub fn scan_folders(
    roots: Vec<PathBuf>,
    state: State<'_, AppState>,
) -> GuiResult<Uuid> {
    info!("Starting folder scan for {} roots", roots.len());
    
    // Validate roots
    for root in &roots {
        if !root.exists() {
            return Err(gui_error!(scan, format!("Root directory does not exist: {}", root.display())));
        }
        if !root.is_dir() {
            return Err(gui_error!(scan, format!("Root path is not a directory: {}", root.display())));
        }
    }
    
    // Create scan session
    let session_id = state.create_scan_session(roots.clone());
    
    // Update session status to running
    state.update_scan_session(session_id, |session| {
        session.status = SessionStatus::Running;
    });
    
    // Perform scan directly (synchronous for simplicity)
    match perform_scan(roots, &state) {
        Ok(results) => {
            state.update_scan_session(session_id, |session| {
                session.status = SessionStatus::Completed;
                session.results = Some(results);
            });
            info!("Scan completed successfully for session {}", session_id);
        }
        Err(error) => {
            state.update_scan_session(session_id, |session| {
                session.status = SessionStatus::Failed;
                session.error = Some(error.to_string());
            });
            warn!("Scan failed for session {}: {}", session_id, error);
        }
    }
    
    debug!("Scan session {} created and started", session_id);
    Ok(session_id)
}

fn perform_scan(roots: Vec<PathBuf>, state: &AppState) -> GuiResult<Vec<FolderHit>> {
    // Get current configuration
    let config = state.config.lock()
        .map_err(|_| gui_error!(scan, "Failed to access configuration"))?
        .clone();
    
    // Create scanner
    let scanner = FolderScanner::new(config.rules, config.options)
        .map_err(|e| gui_error!(scan, format!("Failed to create scanner: {}", e)))?;
    
    // Perform scan
    let results = scanner.scan_roots(&roots)
        .map_err(|e| gui_error!(scan, format!("Scan failed: {}", e)))?;
    
    info!("Scan found {} matching folders", results.len());
    Ok(results)
}

#[tauri::command]
pub async fn get_scan_progress(
    session_id: String,
    state: State<'_, AppState>,
) -> GuiResult<Option<crate::state::ScanSession>> {
    let id = Uuid::parse_str(&session_id)
        .map_err(|_| gui_error!(scan, "Invalid session ID format"))?;
    
    Ok(state.get_scan_session(id))
}

#[tauri::command]
pub async fn cancel_scan(
    session_id: String,
    state: State<'_, AppState>,
) -> GuiResult<()> {
    let id = Uuid::parse_str(&session_id)
        .map_err(|_| gui_error!(scan, "Invalid session ID format"))?;
    
    // Update session status to cancelled
    state.update_scan_session(id, |session| {
        if session.status == SessionStatus::Running {
            session.status = SessionStatus::Cancelled;
        }
    });
    
    // TODO: Implement actual cancellation logic
    // This would need to signal the scanning task to stop
    
    info!("Scan session {} cancelled", id);
    Ok(())
}

#[tauri::command]
pub async fn get_scan_results(
    session_id: String,
    state: State<'_, AppState>,
) -> GuiResult<Vec<FolderHit>> {
    let id = Uuid::parse_str(&session_id)
        .map_err(|_| gui_error!(scan, "Invalid session ID format"))?;
    
    let session = state.get_scan_session(id)
        .ok_or_else(|| gui_error!(session_not_found, session_id))?;
    
    match session.status {
        SessionStatus::Completed => {
            Ok(session.results.unwrap_or_default())
        }
        SessionStatus::Failed => {
            Err(gui_error!(scan, session.error.unwrap_or_else(|| "Unknown scan error".to_string())))
        }
        SessionStatus::Running => {
            Err(gui_error!(scan, "Scan is still running"))
        }
        SessionStatus::Cancelled => {
            Err(gui_error!(scan, "Scan was cancelled"))
        }
        SessionStatus::Created => {
            Err(gui_error!(scan, "Scan has not started yet"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_folders_with_valid_roots() {
        let temp_dir = TempDir::new().unwrap();
        let state = AppState::new();
        
        let roots = vec![temp_dir.path().to_path_buf()];
        let result = scan_folders(roots, State::from(&state)).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scan_folders_with_invalid_root() {
        let state = AppState::new();
        
        let roots = vec![PathBuf::from("/nonexistent/path")];
        let result = scan_folders(roots, State::from(&state)).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_scan_progress_invalid_id() {
        let state = AppState::new();
        
        let result = get_scan_progress("invalid-id".to_string(), State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_scan() {
        let state = AppState::new();
        let session_id = state.create_scan_session(vec![PathBuf::from("/tmp")]);
        
        let result = cancel_scan(session_id.to_string(), State::from(&state)).await;
        assert!(result.is_ok());
        
        let session = state.get_scan_session(session_id).unwrap();
        assert_eq!(session.status, SessionStatus::Cancelled);
    }
}