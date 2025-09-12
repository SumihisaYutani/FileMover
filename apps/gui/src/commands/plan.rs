use tauri::State;
use tracing::{info, debug, warn};
use uuid::Uuid;

use filemover_types::{MovePlan, PlanOptions, NodeChange, ValidationDelta};
use crate::state::{AppState, SessionStatus};
use crate::error::{GuiResult, GuiError, gui_error};

#[tauri::command]
pub fn create_move_plan(
    scan_session_id: Option<String>,
    folder_hits: Option<Vec<filemover_types::FolderHit>>,
    state: State<'_, AppState>,
) -> GuiResult<Uuid> {
    info!("Creating move plan");
    
    let scan_id = if let Some(id_str) = scan_session_id {
        Some(Uuid::parse_str(&id_str)
            .map_err(|_| gui_error!(planning, "Invalid scan session ID format"))?)
    } else {
        None
    };
    
    // Get folder hits either from scan session or direct input
    let hits = if let Some(hits) = folder_hits {
        hits
    } else if let Some(scan_id) = scan_id {
        let scan_session = state.get_scan_session(scan_id)
            .ok_or_else(|| gui_error!(session_not_found, scan_id.to_string()))?;
        
        match scan_session.status {
            SessionStatus::Completed => {
                scan_session.results.unwrap_or_default()
            }
            _ => {
                return Err(gui_error!(planning, "Scan session is not completed"));
            }
        }
    } else {
        return Err(gui_error!(planning, "Either scan_session_id or folder_hits must be provided"));
    };
    
    if hits.is_empty() {
        return Err(gui_error!(planning, "No folder hits to create plan from"));
    }
    
    // Create plan session
    let plan_session_id = state.create_plan_session(scan_id);
    
    // Update session status to running
    state.update_plan_session(plan_session_id, |session| {
        session.status = SessionStatus::Running;
    });
    
    // Perform plan creation directly (synchronous for simplicity)
    match perform_plan_creation(hits, &state) {
        Ok(plan) => {
            state.update_plan_session(plan_session_id, |session| {
                session.status = SessionStatus::Completed;
                session.plan = Some(plan);
            });
            info!("Plan creation completed successfully for session {}", plan_session_id);
        }
        Err(error) => {
            state.update_plan_session(plan_session_id, |session| {
                session.status = SessionStatus::Failed;
                session.error = Some(error.to_string());
            });
            warn!("Plan creation failed for session {}: {}", plan_session_id, error);
        }
    }
    
    debug!("Plan session {} created and started", plan_session_id);
    Ok(plan_session_id)
}

fn perform_plan_creation(
    hits: Vec<filemover_types::FolderHit>,
    state: &AppState,
) -> GuiResult<MovePlan> {
    // Get current configuration
    let config = state.config.lock()
        .map_err(|_| gui_error!(planning, "Failed to access configuration"))?
        .clone();
    
    // Get planner
    let mut planner = state.planner.lock()
        .map_err(|_| gui_error!(planning, "Failed to access planner"))?;
    
    // Create plan
    let plan_options = PlanOptions::default();
    let plan = planner.create_plan(&hits, &config.rules, plan_options)
        .map_err(|e| gui_error!(planning, format!("Failed to create plan: {}", e)))?;
    
    info!("Created plan with {} operations", plan.nodes.len());
    Ok(plan)
}

#[tauri::command]
pub async fn simulate_plan(
    plan_session_id: String,
    state: State<'_, AppState>,
) -> GuiResult<filemover_planner::SimulationReport> {
    let id = Uuid::parse_str(&plan_session_id)
        .map_err(|_| gui_error!(planning, "Invalid plan session ID format"))?;
    
    let plan_session = state.get_plan_session(id)
        .ok_or_else(|| gui_error!(session_not_found, plan_session_id))?;
    
    let plan = plan_session.plan
        .ok_or_else(|| gui_error!(planning, "Plan session has no plan"))?;
    
    // Get planner
    let planner = state.planner.lock()
        .map_err(|_| gui_error!(planning, "Failed to access planner"))?;
    
    // Run simulation
    let simulation = planner.simulate_plan(&plan)
        .map_err(|e| gui_error!(planning, format!("Simulation failed: {}", e)))?;
    
    debug!("Simulation completed for plan {}", id);
    Ok(simulation)
}

#[tauri::command]
pub async fn update_plan_node(
    plan_session_id: String,
    change: NodeChange,
    state: State<'_, AppState>,
) -> GuiResult<ValidationDelta> {
    let id = Uuid::parse_str(&plan_session_id)
        .map_err(|_| gui_error!(planning, "Invalid plan session ID format"))?;
    
    let mut plan_session = state.get_plan_session(id)
        .ok_or_else(|| gui_error!(session_not_found, plan_session_id))?;
    
    let mut plan = plan_session.plan
        .ok_or_else(|| gui_error!(planning, "Plan session has no plan"))?;
    
    // Get planner
    let mut planner = state.planner.lock()
        .map_err(|_| gui_error!(planning, "Failed to access planner"))?;
    
    // Apply change and validate
    let validation_delta = planner.update_plan_with_change(&mut plan, change)
        .map_err(|e| gui_error!(planning, format!("Failed to update plan: {}", e)))?;
    
    // Update the plan in the session
    state.update_plan_session(id, |session| {
        session.plan = Some(plan);
    });
    
    debug!("Plan node updated for session {}", id);
    Ok(validation_delta)
}

#[tauri::command]
pub async fn get_plan_session(
    plan_session_id: String,
    state: State<'_, AppState>,
) -> GuiResult<crate::state::PlanSession> {
    let id = Uuid::parse_str(&plan_session_id)
        .map_err(|_| gui_error!(planning, "Invalid plan session ID format"))?;
    
    state.get_plan_session(id)
        .ok_or_else(|| gui_error!(session_not_found, plan_session_id))
}

#[tauri::command]
pub async fn export_plan(
    plan_session_id: String,
    file_path: String,
    state: State<'_, AppState>,
) -> GuiResult<()> {
    let id = Uuid::parse_str(&plan_session_id)
        .map_err(|_| gui_error!(planning, "Invalid plan session ID format"))?;
    
    let plan_session = state.get_plan_session(id)
        .ok_or_else(|| gui_error!(session_not_found, plan_session_id))?;
    
    let plan = plan_session.plan
        .ok_or_else(|| gui_error!(planning, "Plan session has no plan"))?;
    
    // Serialize and save plan
    let json = serde_json::to_string_pretty(&plan)
        .map_err(|e| gui_error!(planning, format!("Failed to serialize plan: {}", e)))?;
    
    std::fs::write(&file_path, json)
        .map_err(|e| gui_error!(planning, format!("Failed to write plan file: {}", e)))?;
    
    info!("Plan exported to: {}", file_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filemover_types::{FolderHit, Warning};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_create_move_plan_with_hits() {
        let state = AppState::new();
        
        let hits = vec![
            FolderHit {
                path: PathBuf::from("C:\\Test\\folder1"),
                name: "folder1".to_string(),
                matched_rule: None,
                dest_preview: None,
                warnings: vec![],
                size_bytes: Some(1024),
            }
        ];
        
        let result = create_move_plan(None, Some(hits), State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_move_plan_empty_hits() {
        let state = AppState::new();
        
        let result = create_move_plan(None, Some(vec![]), State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_move_plan_no_input() {
        let state = AppState::new();
        
        let result = create_move_plan(None, None, State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_plan_session_invalid_id() {
        let state = AppState::new();
        
        let result = get_plan_session("invalid-id".to_string(), State::from(&state)).await;
        assert!(result.is_err());
    }
}