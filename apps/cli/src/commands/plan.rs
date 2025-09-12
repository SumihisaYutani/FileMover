use std::path::PathBuf;
use anyhow::{Result, Context};
use tracing::{info, debug};
use indicatif::{ProgressBar, ProgressStyle};

use filemover_types::{FolderHit, MovePlan, Rule, PlanOptions};
use filemover_planner::MovePlanner;
use crate::config_manager::ConfigManager;

pub async fn plan_command(
    input_file: Option<PathBuf>,
    output_file: Option<PathBuf>,
    rules_file: Option<PathBuf>,
    config_manager: &ConfigManager,
) -> Result<()> {
    info!("Creating move plan");
    
    // Load scan results
    let input_path = input_file.unwrap_or_else(|| {
        PathBuf::from("scan_results.json")
    });
    
    if !input_path.exists() {
        anyhow::bail!("Input file does not exist: {}", input_path.display());
    }
    
    let folder_hits = load_scan_results(&input_path)
        .context("Failed to load scan results")?;
    
    info!("Loaded {} folder hits from {}", folder_hits.len(), input_path.display());
    
    // Load rules
    let rules = if let Some(rules_path) = rules_file {
        load_rules_from_file(&rules_path)
            .context("Failed to load rules file")?
    } else {
        // Load from config
        let config = config_manager.load_config(None)?;
        config.rules
    };
    
    if rules.is_empty() {
        anyhow::bail!("No rules configured. Specify --rules file or configure in profile.");
    }
    
    info!("Using {} rules for plan generation", rules.len());
    
    // Create progress bar
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    progress.set_message("Generating move plan...");
    
    // Create planner and generate plan
    let mut planner = MovePlanner::new();
    let plan_options = PlanOptions::default();
    
    let plan = planner.create_plan(&folder_hits, &rules, plan_options)
        .context("Failed to generate move plan")?;
    
    progress.finish_with_message("Plan generation completed");
    
    // Display plan summary
    print_plan_summary(&plan);
    
    // Save plan
    let output_path = output_file.unwrap_or_else(|| {
        PathBuf::from(format!(
            "move_plan_{}.json",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
    });
    
    save_move_plan(&plan, &output_path)
        .context("Failed to save move plan")?;
    
    info!("Move plan saved to: {}", output_path.display());
    
    Ok(())
}

fn load_scan_results(path: &PathBuf) -> Result<Vec<FolderHit>> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read scan results file")?;
    
    let hits: Vec<FolderHit> = serde_json::from_str(&content)
        .context("Failed to parse scan results JSON")?;
    
    Ok(hits)
}

fn load_rules_from_file(path: &PathBuf) -> Result<Vec<Rule>> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read rules file")?;
    
    let rules: Vec<Rule> = serde_json::from_str(&content)
        .context("Failed to parse rules JSON")?;
    
    Ok(rules)
}

fn print_plan_summary(plan: &MovePlan) {
    println!("\n=== Move Plan Summary ===");
    println!("Total operations: {}", plan.nodes.len());
    println!("Directories to move: {}", plan.summary.count_dirs);
    
    if let Some(total_bytes) = plan.summary.total_bytes {
        println!("Total data size: {} bytes ({:.2} MB)", 
                 total_bytes, 
                 total_bytes as f64 / (1024.0 * 1024.0));
    }
    
    if plan.summary.cross_volume > 0 {
        println!("Cross-volume operations: {}", plan.summary.cross_volume);
    }
    
    if plan.summary.conflicts > 0 {
        println!("⚠️  Conflicts detected: {}", plan.summary.conflicts);
    }
    
    if plan.summary.warnings > 0 {
        println!("⚠️  Warnings: {}", plan.summary.warnings);
    }
    
    // Show operation breakdown
    let mut op_counts = std::collections::HashMap::new();
    for node in plan.nodes.values() {
        *op_counts.entry(format!("{:?}", node.kind)).or_insert(0) += 1;
    }
    
    println!("\nOperations breakdown:");
    for (op_type, count) in op_counts {
        println!("  {}: {}", op_type, count);
    }
    
    // Show first few operations
    println!("\nFirst 10 operations:");
    let mut shown = 0;
    for node in plan.nodes.values() {
        if shown >= 10 {
            break;
        }
        
        let conflict_info = if !node.conflicts.is_empty() {
            format!(" [⚠️  {} conflicts]", node.conflicts.len())
        } else {
            String::new()
        };
        
        println!("  {}. {:?}: {} -> {}{}",
                 shown + 1,
                 node.kind,
                 node.path_before.display(),
                 node.path_after.display(),
                 conflict_info
        );
        shown += 1;
    }
    
    if plan.nodes.len() > 10 {
        println!("  ... and {} more operations", plan.nodes.len() - 10);
    }
}

fn save_move_plan(plan: &MovePlan, output_path: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(plan)
        .context("Failed to serialize move plan")?;
    
    std::fs::write(output_path, json)
        .context("Failed to write move plan file")?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use filemover_types::{PatternSpec, ConflictPolicy, Warning};

    #[tokio::test]
    async fn test_plan_command_with_missing_input() {
        let config_manager = ConfigManager::new(None).unwrap();
        
        let result = plan_command(
            Some(PathBuf::from("nonexistent.json")),
            None,
            None,
            &config_manager
        ).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_load_scan_results() {
        let temp_dir = TempDir::new().unwrap();
        let scan_file = temp_dir.path().join("scan_results.json");
        
        // Create test scan results
        let hits = vec![
            FolderHit {
                path: PathBuf::from("C:\\Test\\folder1"),
                name: "folder1".to_string(),
                matched_rule: None,
                dest_preview: Some(PathBuf::from("D:\\Archive\\folder1")),
                warnings: vec![Warning::LongPath],
                size_bytes: Some(1024),
            }
        ];
        
        let json = serde_json::to_string_pretty(&hits).unwrap();
        std::fs::write(&scan_file, json).unwrap();
        
        let loaded = load_scan_results(&scan_file).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "folder1");
    }
    
    #[tokio::test]
    async fn test_load_rules_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let rules_file = temp_dir.path().join("rules.json");
        
        // Create test rules
        let rules = vec![
            Rule::new(
                PatternSpec::new_glob("test*"),
                PathBuf::from("D:\\Archive"),
                "{name}".to_string()
            )
        ];
        
        let json = serde_json::to_string_pretty(&rules).unwrap();
        std::fs::write(&rules_file, json).unwrap();
        
        let loaded = load_rules_from_file(&rules_file).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].template, "{name}");
    }
}