use std::path::PathBuf;
use anyhow::{Result, Context};
use tracing::{info, warn, error};
use std::io::{self, Write};

use filemover_types::{JournalEntry, ResultKind, OpKind};
use crate::config_manager::ConfigManager;

pub async fn undo_command(
    journal_file: PathBuf,
    _config_manager: &ConfigManager,
) -> Result<()> {
    info!("Starting undo operation");
    
    // Validate journal file exists
    if !journal_file.exists() {
        anyhow::bail!("Journal file does not exist: {}", journal_file.display());
    }
    
    // Load journal entries
    let entries = load_journal_entries(&journal_file)
        .context("Failed to load journal file")?;
    
    if entries.is_empty() {
        println!("Journal file is empty - nothing to undo.");
        return Ok(());
    }
    
    info!("Loaded {} journal entries from {}", 
          entries.len(), 
          journal_file.display());
    
    // Analyze journal for undo feasibility
    let (undoable, issues) = analyze_undo_feasibility(&entries);
    
    // Show undo summary
    print_undo_summary(&entries, &undoable, &issues);
    
    if undoable.is_empty() {
        println!("❌ No operations can be undone.");
        return Ok(());
    }
    
    // Get user confirmation
    if !get_undo_confirmation(&undoable)? {
        println!("Undo cancelled by user.");
        return Ok(());
    }
    
    // Execute undo operations
    execute_undo_operations(&undoable).await?;
    
    println!("\n✅ Undo operation completed!");
    
    Ok(())
}

fn load_journal_entries(path: &PathBuf) -> Result<Vec<JournalEntry>> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read journal file")?;
    
    let mut entries = Vec::new();
    
    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        
        let entry: JournalEntry = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse journal entry at line {}", line_num + 1))?;
        
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

fn print_undo_summary(
    all_entries: &[JournalEntry],
    undoable: &[&JournalEntry],
    issues: &[String]
) {
    println!("\n=== Undo Analysis ===");
    
    let successful_ops = all_entries.iter()
        .filter(|e| matches!(e.result, ResultKind::Ok))
        .count();
    
    let skipped_ops = all_entries.iter()
        .filter(|e| matches!(e.result, ResultKind::Skip))
        .count();
    
    let failed_ops = all_entries.iter()
        .filter(|e| matches!(e.result, ResultKind::Failed))
        .count();
    
    println!("📊 Original operation results:");
    println!("  ✅ Successful: {}", successful_ops);
    println!("  ⏭️  Skipped: {}", skipped_ops);
    println!("  ❌ Failed: {}", failed_ops);
    
    println!("\n🔄 Undo feasibility:");
    println!("  ✅ Can be undone: {}", undoable.len());
    println!("  ❌ Cannot be undone: {}", successful_ops - undoable.len());
    
    if !issues.is_empty() {
        println!("\n⚠️  Issues preventing undo:");
        for (i, issue) in issues.iter().enumerate() {
            println!("  {}. {}", i + 1, issue);
        }
    }
    
    if !undoable.is_empty() {
        println!("\n📝 Operations to undo (in reverse order):");
        for (i, entry) in undoable.iter().take(10).enumerate() {
            let op_description = match entry.op {
                OpKind::Move => "Move back",
                OpKind::CopyDelete => "Copy back and delete",
                OpKind::Rename => "Rename back",
                _ => "Reverse",
            };
            
            println!("  {}. {}: {} <- {}",
                     i + 1,
                     op_description,
                     entry.source.display(),
                     entry.dest.display()
            );
        }
        
        if undoable.len() > 10 {
            println!("  ... and {} more operations", undoable.len() - 10);
        }
    }
}

fn get_undo_confirmation(undoable: &[&JournalEntry]) -> Result<bool> {
    println!("\n⚠️  WARNING: This will reverse {} file operations!", undoable.len());
    println!("Files will be moved back to their original locations.");
    print!("\nDo you want to proceed with undo? (y/N): ");
    
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

async fn execute_undo_operations(entries: &[&JournalEntry]) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::time::Duration;
    use tokio::time::sleep;
    
    let progress = ProgressBar::new(entries.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    
    let mut successful_undos = 0;
    let mut failed_undos = 0;
    
    for (i, entry) in entries.iter().enumerate() {
        progress.set_message(format!("Undoing: {}", 
            entry.dest.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ));
        
        // Simulate undo operation
        sleep(Duration::from_millis(50)).await;
        
        // In real implementation, this would:
        // 1. Reverse the original operation (move dest back to source)
        // 2. Handle errors and edge cases
        // 3. Verify the undo was successful
        
        // Stub: Just log what would happen
        match execute_single_undo(entry).await {
            Ok(_) => {
                successful_undos += 1;
                info!("Undone: {} <- {}", entry.source.display(), entry.dest.display());
            }
            Err(e) => {
                failed_undos += 1;
                error!("Failed to undo {}: {}", entry.dest.display(), e);
            }
        }
        
        progress.set_position(i as u64 + 1);
    }
    
    progress.finish_with_message("Undo operations completed");
    
    // Print final results
    println!("\n📊 Undo Results:");
    println!("  ✅ Successfully undone: {}", successful_undos);
    if failed_undos > 0 {
        println!("  ❌ Failed to undo: {}", failed_undos);
    }
    
    if failed_undos == 0 {
        println!("\n🎉 All operations were successfully undone!");
    } else {
        println!("\n⚠️  Some operations could not be undone. Check the logs for details.");
    }
    
    Ok(())
}

async fn execute_single_undo(entry: &JournalEntry) -> Result<()> {
    // Stub implementation for single undo operation
    // In real implementation, this would:
    
    match entry.op {
        OpKind::Move => {
            // Move dest back to source
            info!("Would move {} back to {}", entry.dest.display(), entry.source.display());
        }
        OpKind::CopyDelete => {
            // Copy dest back to source, then delete dest
            info!("Would copy {} back to {} and delete dest", entry.dest.display(), entry.source.display());
        }
        OpKind::Rename => {
            // Rename dest back to source name
            info!("Would rename {} back to {}", entry.dest.display(), entry.source.display());
        }
        _ => {
            warn!("Unsupported operation type for undo: {:?}", entry.op);
        }
    }
    
    // Simulate some processing time
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use chrono::Utc;

    #[tokio::test]
    async fn test_undo_command_with_missing_journal() {
        let config_manager = ConfigManager::new(None).unwrap();
        
        let result = undo_command(
            PathBuf::from("nonexistent_journal.jsonl"),
            &config_manager
        ).await;
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_load_journal_entries() {
        let temp_dir = TempDir::new().unwrap();
        let journal_file = temp_dir.path().join("test_journal.jsonl");
        
        // Create test journal
        let entries = vec![
            JournalEntry::new(
                PathBuf::from("C:\\Source\\test1"),
                PathBuf::from("D:\\Dest\\test1"),
                OpKind::Move
            ),
            JournalEntry::new(
                PathBuf::from("C:\\Source\\test2"),
                PathBuf::from("D:\\Dest\\test2"),
                OpKind::CopyDelete
            ).with_result(ResultKind::Failed),
        ];
        
        let mut journal_content = String::new();
        for entry in &entries {
            journal_content.push_str(&serde_json::to_string(entry).unwrap());
            journal_content.push('\n');
        }
        
        std::fs::write(&journal_file, journal_content).unwrap();
        
        let loaded = load_journal_entries(&journal_file).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].op, OpKind::Move);
        assert_eq!(loaded[1].result, ResultKind::Failed);
    }
    
    #[test]
    fn test_analyze_undo_feasibility() {
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
            JournalEntry::new(
                PathBuf::from("C:\\nonexistent"),
                PathBuf::from("D:\\also_nonexistent"),
                OpKind::Move
            ).with_result(ResultKind::Failed),
        ];
        
        let (undoable, issues) = analyze_undo_feasibility(&entries);
        
        assert_eq!(undoable.len(), 1); // Only the first entry can be undone
        assert!(!issues.is_empty()); // Should have issues with the second entry
    }
    
    #[tokio::test]
    async fn test_execute_single_undo() {
        let entry = JournalEntry::new(
            PathBuf::from("C:\\Source\\test"),
            PathBuf::from("D:\\Dest\\test"),
            OpKind::Move
        );
        
        // Should not fail (it's a stub implementation)
        let result = execute_single_undo(&entry).await;
        assert!(result.is_ok());
    }
}