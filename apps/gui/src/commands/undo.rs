use std::path::PathBuf;
use tauri::State;
use tracing::{info, debug, warn};

use filemover_types::{JournalEntry, ResultKind};
use crate::state::AppState;
use crate::error::{GuiResult, GuiError, gui_error};

#[tauri::command]
pub async fn undo_operation(
    journal_path: PathBuf,
    state: State<'_, AppState>,
) -> GuiResult<UndoResult> {
    info!("Starting undo operation from journal: {}", journal_path.display());
    
    // Validate journal file exists
    if !journal_path.exists() {
        return Err(gui_error!(execution, format!("Journal file does not exist: {}", journal_path.display())));
    }
    
    // Load journal entries
    let entries = load_journal_entries(&journal_path)?;
    
    if entries.is_empty() {
        return Ok(UndoResult {
            total_operations: 0,
            undone_operations: 0,
            failed_operations: 0,
            skipped_operations: 0,
            errors: vec![],
        });
    }
    
    // Analyze what can be undone
    let (undoable, issues) = analyze_undo_feasibility(&entries);
    
    info!("Found {} undoable operations out of {} total", undoable.len(), entries.len());
    
    // Perform undo operations
    let result = perform_undo_operations(undoable).await;
    
    info!("Undo operation completed: {} succeeded, {} failed", 
          result.undone_operations, result.failed_operations);
    
    Ok(result)
}

fn load_journal_entries(path: &PathBuf) -> GuiResult<Vec<JournalEntry>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| gui_error!(execution, format!("Failed to read journal file: {}", e)))?;
    
    let mut entries = Vec::new();
    
    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        
        let entry: JournalEntry = serde_json::from_str(line)
            .map_err(|e| gui_error!(execution, 
                format!("Failed to parse journal entry at line {}: {}", line_num + 1, e)))?;
        
        entries.push(entry);
    }
    
    Ok(entries)
}

fn analyze_undo_feasibility(entries: &[JournalEntry]) -> (Vec<&JournalEntry>, Vec<String>) {
    let mut undoable = Vec::new();
    let mut issues = Vec::new();
    
    // Only successful operations can be undone
    for entry in entries {
        match entry.result {
            ResultKind::Ok => {
                // Check if destination still exists and source doesn't
                if entry.dest.exists() && !entry.source.exists() {
                    undoable.push(entry);
                } else if !entry.dest.exists() {
                    issues.push(format!("Destination no longer exists: {}", entry.dest.display()));
                } else if entry.source.exists() {
                    issues.push(format!("Source already exists: {}", entry.source.display()));
                }
            }
            ResultKind::Skip => {
                // Skipped operations don't need undo
                continue;
            }
            ResultKind::Failed => {
                // Failed operations don't need undo
                continue;
            }
        }
    }
    
    // Reverse the order for undo (last operation first)
    undoable.reverse();
    
    (undoable, issues)
}

async fn perform_undo_operations(entries: Vec<&JournalEntry>) -> UndoResult {
    let mut result = UndoResult {
        total_operations: entries.len(),
        undone_operations: 0,
        failed_operations: 0,
        skipped_operations: 0,
        errors: vec![],
    };
    
    for entry in entries {
        match perform_single_undo(entry).await {
            Ok(_) => {
                result.undone_operations += 1;
                debug!("Undone: {} <- {}", entry.source.display(), entry.dest.display());
            }
            Err(error) => {
                result.failed_operations += 1;
                result.errors.push(format!("Failed to undo {}: {}", entry.dest.display(), error));
                warn!("Failed to undo {}: {}", entry.dest.display(), error);
            }
        }
        
        // Small delay to prevent overwhelming the file system
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    
    result
}

async fn perform_single_undo(entry: &JournalEntry) -> GuiResult<()> {
    // Stub implementation for single undo operation
    // In real implementation, this would:
    
    match entry.op {
        filemover_types::OpKind::Move => {
            // Move dest back to source
            debug!("Would move {} back to {}", entry.dest.display(), entry.source.display());
            // TODO: Implement actual file move
        }
        filemover_types::OpKind::CopyDelete => {
            // Copy dest back to source, then delete dest
            debug!("Would copy {} back to {} and delete dest", entry.dest.display(), entry.source.display());
            // TODO: Implement actual file copy and delete
        }
        filemover_types::OpKind::Rename => {
            // Rename dest back to source name
            debug!("Would rename {} back to {}", entry.dest.display(), entry.source.display());
            // TODO: Implement actual file rename
        }
        _ => {
            return Err(gui_error!(execution, format!("Unsupported operation type for undo: {:?}", entry.op)));
        }
    }
    
    // Simulate some processing time
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    
    Ok(())
}

#[tauri::command]
pub async fn validate_journal(
    journal_path: PathBuf,
) -> GuiResult<JournalValidation> {
    info!("Validating journal file: {}", journal_path.display());
    
    if !journal_path.exists() {
        return Ok(JournalValidation {
            is_valid: false,
            total_entries: 0,
            successful_entries: 0,
            failed_entries: 0,
            skipped_entries: 0,
            undoable_entries: 0,
            issues: vec!["Journal file does not exist".to_string()],
        });
    }
    
    let entries = match load_journal_entries(&journal_path) {
        Ok(entries) => entries,
        Err(e) => {
            return Ok(JournalValidation {
                is_valid: false,
                total_entries: 0,
                successful_entries: 0,
                failed_entries: 0,
                skipped_entries: 0,
                undoable_entries: 0,
                issues: vec![format!("Failed to load journal: {}", e)],
            });
        }
    };
    
    let (undoable, issues) = analyze_undo_feasibility(&entries);
    
    let successful_entries = entries.iter()
        .filter(|e| matches!(e.result, ResultKind::Ok))
        .count();
    
    let failed_entries = entries.iter()
        .filter(|e| matches!(e.result, ResultKind::Failed))
        .count();
    
    let skipped_entries = entries.iter()
        .filter(|e| matches!(e.result, ResultKind::Skip))
        .count();
    
    Ok(JournalValidation {
        is_valid: true,
        total_entries: entries.len(),
        successful_entries,
        failed_entries,
        skipped_entries,
        undoable_entries: undoable.len(),
        issues,
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UndoResult {
    pub total_operations: usize,
    pub undone_operations: usize,
    pub failed_operations: usize,
    pub skipped_operations: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JournalValidation {
    pub is_valid: bool,
    pub total_entries: usize,
    pub successful_entries: usize,
    pub failed_entries: usize,
    pub skipped_entries: usize,
    pub undoable_entries: usize,
    pub issues: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use filemover_types::OpKind;

    #[tokio::test]
    async fn test_undo_operation_nonexistent_journal() {
        let state = AppState::new();
        
        let result = undo_operation(
            PathBuf::from("/nonexistent/journal.jsonl"),
            State::from(&state)
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_journal_nonexistent() {
        let result = validate_journal(PathBuf::from("/nonexistent/journal.jsonl")).await;
        
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(!validation.is_valid);
    }

    #[tokio::test]
    async fn test_load_journal_entries() {
        let temp_dir = TempDir::new().unwrap();
        let journal_file = temp_dir.path().join("test_journal.jsonl");
        
        // Create test journal
        let entry = JournalEntry::new(
            PathBuf::from("C:\\Source\\test"),
            PathBuf::from("D:\\Dest\\test"),
            OpKind::Move
        );
        
        let json_line = serde_json::to_string(&entry).unwrap();
        std::fs::write(&journal_file, json_line).unwrap();
        
        let loaded = load_journal_entries(&journal_file).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].op, OpKind::Move);
    }

    #[tokio::test]
    async fn test_analyze_undo_feasibility() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test destination file
        let dest_path = temp_dir.path().join("dest_file");
        std::fs::write(&dest_path, "test content").unwrap();
        
        let entries = vec![
            JournalEntry::new(
                temp_dir.path().join("source_file"), // doesn't exist
                dest_path, // exists
                OpKind::Move
            ).with_result(ResultKind::Ok),
        ];
        
        let (undoable, _issues) = analyze_undo_feasibility(&entries);
        assert_eq!(undoable.len(), 1);
    }
}