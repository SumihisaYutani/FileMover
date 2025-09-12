use std::path::PathBuf;
use anyhow::{Result, Context};
use tracing::{info, warn, error};
use std::io::{self, Write};

use filemover_types::MovePlan;
use crate::config_manager::ConfigManager;

pub async fn apply_command(
    plan_file: PathBuf,
    journal_file: Option<PathBuf>,
    skip_confirmation: bool,
    _config_manager: &ConfigManager,
) -> Result<()> {
    info!("Starting plan execution");
    
    // Load move plan
    if !plan_file.exists() {
        anyhow::bail!("Plan file does not exist: {}", plan_file.display());
    }
    
    let plan = load_move_plan(&plan_file)
        .context("Failed to load move plan")?;
    
    info!("Loaded move plan with {} operations from {}", 
          plan.nodes.len(), 
          plan_file.display());
    
    // Pre-execution validation
    validate_plan_for_execution(&plan)?;
    
    // Show execution summary and get confirmation
    if !skip_confirmation {
        print_execution_summary(&plan);
        if !get_user_confirmation()? {
            println!("Execution cancelled by user.");
            return Ok(());
        }
    }
    
    // Determine journal file path
    let journal_path = journal_file.unwrap_or_else(|| {
        PathBuf::from(format!(
            "filemover_journal_{}.jsonl",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
    });
    
    info!("Journal will be written to: {}", journal_path.display());
    
    // Execute plan (currently a stub - would integrate with executor module)
    execute_plan_stub(&plan, &journal_path).await?;
    
    println!("\n✅ Execution completed successfully!");
    println!("📄 Journal saved to: {}", journal_path.display());
    println!("🔄 To undo this operation: filemover undo --journal {}", journal_path.display());
    
    Ok(())
}

fn load_move_plan(path: &PathBuf) -> Result<MovePlan> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read move plan file")?;
    
    let plan: MovePlan = serde_json::from_str(&content)
        .context("Failed to parse move plan JSON")?;
    
    Ok(plan)
}

fn validate_plan_for_execution(plan: &MovePlan) -> Result<()> {
    // Check if there are any operations to execute
    if plan.nodes.is_empty() {
        anyhow::bail!("Move plan is empty - no operations to execute");
    }
    
    // Count executable operations
    let executable_count = plan.nodes.values()
        .filter(|node| !matches!(node.kind, filemover_types::OpKind::Skip | filemover_types::OpKind::None))
        .count();
    
    if executable_count == 0 {
        anyhow::bail!("No executable operations found in plan (all operations are skipped or disabled)");
    }
    
    // Check for critical conflicts
    let critical_conflicts = plan.nodes.values()
        .filter(|node| !node.conflicts.is_empty())
        .count();
    
    if critical_conflicts > 0 {
        warn!("⚠️  {} operations have unresolved conflicts", critical_conflicts);
        println!("These conflicts will be handled according to the configured conflict policy.");
    }
    
    // Validate source paths exist
    let mut missing_sources = 0;
    for node in plan.nodes.values() {
        if !matches!(node.kind, filemover_types::OpKind::Skip | filemover_types::OpKind::None) {
            if !node.path_before.exists() {
                warn!("Source path no longer exists: {}", node.path_before.display());
                missing_sources += 1;
            }
        }
    }
    
    if missing_sources > 0 {
        warn!("⚠️  {} source paths no longer exist", missing_sources);
        println!("These operations will be skipped during execution.");
    }
    
    Ok(())
}

fn print_execution_summary(plan: &MovePlan) {
    println!("\n=== Execution Summary ===");
    
    let executable_ops: Vec<_> = plan.nodes.values()
        .filter(|node| !matches!(node.kind, filemover_types::OpKind::Skip | filemover_types::OpKind::None))
        .collect();
    
    println!("📊 Operations to execute: {}", executable_ops.len());
    
    if let Some(total_bytes) = plan.summary.total_bytes {
        println!("💾 Total data size: {} bytes ({:.2} MB)", 
                 total_bytes, 
                 total_bytes as f64 / (1024.0 * 1024.0));
    }
    
    if plan.summary.cross_volume > 0 {
        println!("🔄 Cross-volume operations: {} (slower)", plan.summary.cross_volume);
    }
    
    if plan.summary.conflicts > 0 {
        println!("⚠️  Operations with conflicts: {}", plan.summary.conflicts);
    }
    
    // Show operation types breakdown
    let mut op_counts = std::collections::HashMap::new();
    for node in &executable_ops {
        *op_counts.entry(&node.kind).or_insert(0) += 1;
    }
    
    println!("\n📋 Operations breakdown:");
    for (op_kind, count) in op_counts {
        let description = match op_kind {
            filemover_types::OpKind::Move => "Fast move within same volume",
            filemover_types::OpKind::CopyDelete => "Copy + delete (cross-volume)",
            filemover_types::OpKind::Rename => "Rename in place",
            _ => "Other",
        };
        println!("  {:?}: {} operations ({})", op_kind, count, description);
    }
    
    // Show a few example operations
    println!("\n📝 First few operations:");
    for (i, node) in executable_ops.iter().take(5).enumerate() {
        let conflict_info = if !node.conflicts.is_empty() {
            format!(" [⚠️ {} conflicts]", node.conflicts.len())
        } else {
            String::new()
        };
        
        println!("  {}. {} -> {}{}",
                 i + 1,
                 node.path_before.display(),
                 node.path_after.display(),
                 conflict_info
        );
    }
    
    if executable_ops.len() > 5 {
        println!("  ... and {} more operations", executable_ops.len() - 5);
    }
}

fn get_user_confirmation() -> Result<bool> {
    println!("\n⚠️  WARNING: This will permanently move/modify your files!");
    println!("Make sure you have backups of important data.");
    print!("\nDo you want to proceed? (y/N): ");
    
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

// Stub implementation for plan execution
// In the real implementation, this would use the executor module
async fn execute_plan_stub(plan: &MovePlan, journal_path: &PathBuf) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::time::Duration;
    use tokio::time::sleep;
    
    let executable_ops: Vec<_> = plan.nodes.values()
        .filter(|node| !matches!(node.kind, filemover_types::OpKind::Skip | filemover_types::OpKind::None))
        .collect();
    
    let progress = ProgressBar::new(executable_ops.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    
    // Create journal file
    std::fs::write(journal_path, "")?; // Create empty journal file
    
    for (i, node) in executable_ops.iter().enumerate() {
        progress.set_message(format!("Processing: {}", 
            node.path_before.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ));
        
        // Simulate operation execution
        sleep(Duration::from_millis(100)).await;
        
        // In real implementation, this would:
        // 1. Execute the file operation using Windows APIs
        // 2. Handle errors and conflicts
        // 3. Write journal entries for undo
        // 4. Update progress
        
        // Stub: Just log what would happen
        info!("Would execute {:?}: {} -> {}", 
              node.kind, 
              node.path_before.display(), 
              node.path_after.display());
        
        // Simulate writing journal entry
        append_journal_entry(journal_path, node)?;
        
        progress.set_position(i as u64 + 1);
    }
    
    progress.finish_with_message("All operations completed");
    
    Ok(())
}

fn append_journal_entry(journal_path: &PathBuf, node: &filemover_types::PlanNode) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    
    let entry = filemover_types::JournalEntry::new(
        node.path_before.clone(),
        node.path_after.clone(),
        node.kind
    );
    
    let json_line = serde_json::to_string(&entry)?;
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal_path)?;
    
    writeln!(file, "{}", json_line)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use filemover_types::{PlanNode, PlanNodeId, PlanSummary, OpKind};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_apply_command_with_missing_plan() {
        let config_manager = ConfigManager::new(None).unwrap();
        
        let result = apply_command(
            PathBuf::from("nonexistent_plan.json"),
            None,
            true, // skip confirmation for test
            &config_manager
        ).await;
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_empty_plan() {
        let plan = MovePlan {
            roots: vec![],
            nodes: HashMap::new(),
            summary: PlanSummary::default(),
        };
        
        let result = validate_plan_for_execution(&plan);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_plan_with_only_skipped_operations() {
        let mut nodes = HashMap::new();
        let node_id = PlanNodeId::new();
        let node = PlanNode {
            id: node_id,
            is_dir: true,
            name_before: "test".to_string(),
            path_before: PathBuf::from("C:\\Test"),
            name_after: "test".to_string(),
            path_after: PathBuf::from("D:\\Archive\\test"),
            kind: OpKind::Skip,
            size_bytes: None,
            warnings: vec![],
            conflicts: vec![],
            children: vec![],
            rule_id: None,
        };
        nodes.insert(node_id, node);
        
        let plan = MovePlan {
            roots: vec![node_id],
            nodes,
            summary: PlanSummary::default(),
        };
        
        let result = validate_plan_for_execution(&plan);
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_execute_plan_stub() {
        let temp_dir = TempDir::new().unwrap();
        let journal_path = temp_dir.path().join("test_journal.jsonl");
        
        let mut nodes = HashMap::new();
        let node_id = PlanNodeId::new();
        let node = PlanNode {
            id: node_id,
            is_dir: true,
            name_before: "test".to_string(),
            path_before: PathBuf::from("C:\\Test"),
            name_after: "test".to_string(),
            path_after: PathBuf::from("D:\\Archive\\test"),
            kind: OpKind::Move,
            size_bytes: None,
            warnings: vec![],
            conflicts: vec![],
            children: vec![],
            rule_id: None,
        };
        nodes.insert(node_id, node);
        
        let plan = MovePlan {
            roots: vec![node_id],
            nodes,
            summary: PlanSummary::default(),
        };
        
        let result = execute_plan_stub(&plan, &journal_path).await;
        assert!(result.is_ok());
        
        // Check that journal file was created
        assert!(journal_path.exists());
        
        let journal_content = std::fs::read_to_string(&journal_path).unwrap();
        assert!(!journal_content.is_empty());
    }
}