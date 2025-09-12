use std::path::PathBuf;
use tauri::State;
use tracing::{info, debug, warn};
use uuid::Uuid;

use crate::state::{AppState, SessionStatus};
use crate::error::{GuiResult, GuiError, gui_error};

#[tauri::command]
pub async fn execute_plan(
    plan_session_id: String,
    journal_path: Option<PathBuf>,
    state: State<'_, AppState>,
) -> GuiResult<Uuid> {
    info!("Starting plan execution");
    
    let plan_id = Uuid::parse_str(&plan_session_id)
        .map_err(|_| gui_error!(execution, "Invalid plan session ID format"))?;
    
    // Verify plan session exists and has a completed plan
    let plan_session = state.get_plan_session(plan_id)
        .ok_or_else(|| gui_error!(session_not_found, plan_session_id))?;
    
    let _plan = plan_session.plan
        .ok_or_else(|| gui_error!(execution, "Plan session has no plan"))?;
    
    // Create execution session
    let execution_session_id = state.create_execution_session(plan_id);
    
    // Determine journal path
    let journal_file = journal_path.unwrap_or_else(|| {
        PathBuf::from(format!(
            "filemover_journal_{}.jsonl",
"default_journal"
        ))
    });
    
    // Clone necessary data for the async task
    let state_clone = state.inner().clone();
    
    // Start execution in background
    tokio::spawn(async move {
        // Update session status to running
        state_clone.update_execution_session(execution_session_id, |session| {
            session.status = SessionStatus::Running;
            session.journal_path = Some(journal_file.clone());
        });
        
        match perform_execution(plan_id, journal_file, &state_clone).await {
            Ok(_) => {
                state_clone.update_execution_session(execution_session_id, |session| {
                    session.status = SessionStatus::Completed;
                });
                info!("Execution completed successfully for session {}", execution_session_id);
            }
            Err(error) => {
                state_clone.update_execution_session(execution_session_id, |session| {
                    session.status = SessionStatus::Failed;
                    session.error = Some(error.to_string());
                });
                warn!("Execution failed for session {}: {}", execution_session_id, error);
            }
        }
    });
    
    debug!("Execution session {} created and started", execution_session_id);
    Ok(execution_session_id)
}

async fn perform_execution(
    plan_id: Uuid,
    journal_path: PathBuf,
    state: &AppState,
) -> GuiResult<()> {
    // Get the plan
    let plan_session = state.get_plan_session(plan_id)
        .ok_or_else(|| gui_error!(execution, "Plan session not found"))?;
    
    let plan = plan_session.plan
        .ok_or_else(|| gui_error!(execution, "Plan session has no plan"))?;
    
    // TODO: Implement actual execution using the executor module
    // For now, this is a stub that simulates execution
    
    info!("Simulating execution of {} operations", plan.nodes.len());
    
    // Create journal file
    std::fs::write(&journal_path, "")
        .map_err(|e| gui_error!(execution, format!("Failed to create journal file: {}", e)))?;
    
    // Simulate processing each operation
    for (i, (_node_id, node)) in plan.nodes.iter().enumerate() {
        // Simulate progress update
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Log what would be done
        debug!("Would execute {:?}: {} -> {}", 
               node.kind, 
               node.path_before.display(), 
               node.path_after.display());
        
        // Write journal entry
        let entry = filemover_types::JournalEntry::new(
            node.path_before.clone(),
            node.path_after.clone(),
            node.kind
        );
        
        let json_line = serde_json::to_string(&entry)
            .map_err(|e| gui_error!(execution, format!("Failed to serialize journal entry: {}", e)))?;
        
        std::fs::write(&journal_path, format!("{}\n", json_line))
            .map_err(|e| gui_error!(execution, format!("Failed to write journal entry: {}", e)))?;
        
        // Update progress (would be more sophisticated in real implementation)
        let progress = filemover_types::Progress::new(plan.nodes.len() as u64, plan.summary.total_bytes);
        // TODO: Update execution session with progress
    }
    
    info!("Execution simulation completed, journal written to: {}", journal_path.display());
    Ok(())
}

#[tauri::command]
pub async fn get_execution_progress(
    execution_session_id: String,
    state: State<'_, AppState>,
) -> GuiResult<Option<crate::state::ExecutionSession>> {
    let id = Uuid::parse_str(&execution_session_id)
        .map_err(|_| gui_error!(execution, "Invalid execution session ID format"))?;
    
    Ok(state.get_execution_session(id))
}

#[tauri::command]
pub async fn cancel_execution(
    execution_session_id: String,
    state: State<'_, AppState>,
) -> GuiResult<()> {
    let id = Uuid::parse_str(&execution_session_id)
        .map_err(|_| gui_error!(execution, "Invalid execution session ID format"))?;
    
    // Update session status to cancelled
    state.update_execution_session(id, |session| {
        if session.status == SessionStatus::Running {
            session.status = SessionStatus::Cancelled;
        }
    });
    
    // TODO: Implement actual cancellation logic
    // This would need to signal the execution task to stop
    
    info!("Execution session {} cancelled", id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filemover_types::{MovePlan, PlanSummary};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_execute_plan_invalid_session() {
        let state = AppState::new();
        
        let result = execute_plan(
            "invalid-id".to_string(),
            None,
            State::from(&state)
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_plan_no_plan() {
        let state = AppState::new();
        let plan_session_id = state.create_plan_session(None);
        
        let result = execute_plan(
            plan_session_id.to_string(),
            None,
            State::from(&state)
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_execution_progress_invalid_id() {
        let state = AppState::new();
        
        let result = get_execution_progress("invalid-id".to_string(), State::from(&state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_execution() {
        let state = AppState::new();
        let plan_id = Uuid::new_v4();
        let execution_session_id = state.create_execution_session(plan_id);
        
        let result = cancel_execution(execution_session_id.to_string(), State::from(&state)).await;
        assert!(result.is_ok());
        
        let session = state.get_execution_session(execution_session_id).unwrap();
        assert_eq!(session.status, SessionStatus::Cancelled);
    }
}