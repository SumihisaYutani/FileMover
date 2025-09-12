use std::path::PathBuf;
use anyhow::{Result, Context};
use tracing::{info, warn};
use indicatif::{ProgressBar, ProgressStyle};

use filemover_types::{MovePlan, OpKind};
use filemover_planner::{MovePlanner, SimulationReport};
use crate::config_manager::ConfigManager;

pub async fn dry_run_command(
    plan_file: PathBuf,
    _config_manager: &ConfigManager,
) -> Result<()> {
    info!("Starting dry-run simulation");
    
    // Load move plan
    if !plan_file.exists() {
        anyhow::bail!("Plan file does not exist: {}", plan_file.display());
    }
    
    let plan = load_move_plan(&plan_file)
        .context("Failed to load move plan")?;
    
    info!("Loaded move plan with {} operations from {}", 
          plan.nodes.len(), 
          plan_file.display());
    
    // Create progress bar
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    progress.set_message("Running simulation...");
    
    // Run simulation
    let planner = MovePlanner::new();
    let simulation = planner.simulate_plan(&plan)
        .context("Failed to run plan simulation")?;
    
    progress.finish_with_message("Simulation completed");
    
    // Display simulation results
    print_simulation_results(&plan, &simulation);
    
    // Analyze potential issues
    analyze_potential_issues(&plan);
    
    // Provide recommendations
    provide_recommendations(&plan, &simulation);
    
    Ok(())
}

fn load_move_plan(path: &PathBuf) -> Result<MovePlan> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read move plan file")?;
    
    let plan: MovePlan = serde_json::from_str(&content)
        .context("Failed to parse move plan JSON")?;
    
    Ok(plan)
}

fn print_simulation_results(plan: &MovePlan, simulation: &SimulationReport) {
    println!("\n=== Dry-Run Simulation Results ===");
    
    // Overall statistics
    println!("📊 Execution Forecast:");
    println!("  ✅ Expected successful operations: {}", simulation.success_estimate);
    println!("  ⚠️  Operations with conflicts: {}", simulation.conflicts_remaining);
    println!("  ⏭️  Operations to skip: {}", simulation.skipped_count);
    println!("  ⏱️  Estimated duration: {:?}", simulation.estimated_duration);
    
    // Operation breakdown
    let mut op_stats = std::collections::HashMap::new();
    for node in plan.nodes.values() {
        *op_stats.entry(&node.kind).or_insert(0) += 1;
    }
    
    println!("\n📋 Operation Types:");
    for (op_kind, count) in &op_stats {
        let icon = match op_kind {
            OpKind::Move => "📁",
            OpKind::CopyDelete => "📂",
            OpKind::Rename => "✏️",
            OpKind::Skip => "⏭️",
            OpKind::None => "❌",
        };
        println!("  {} {:?}: {} operations", icon, op_kind, count);
    }
    
    // Data transfer info
    if let Some(total_bytes) = plan.summary.total_bytes {
        println!("\n💾 Data Transfer:");
        println!("  Total size: {} bytes ({:.2} MB)", 
                 total_bytes, 
                 total_bytes as f64 / (1024.0 * 1024.0));
        
        if plan.summary.cross_volume > 0 {
            println!("  Cross-volume operations: {} (slower)", plan.summary.cross_volume);
        }
    }
}

fn analyze_potential_issues(plan: &MovePlan) {
    println!("\n🔍 Potential Issues Analysis:");
    
    let mut has_issues = false;
    
    // Check for conflicts
    if plan.summary.conflicts > 0 {
        println!("  ⚠️  {} operations have unresolved conflicts", plan.summary.conflicts);
        
        // Show some conflict examples
        let mut conflict_examples = 0;
        for node in plan.nodes.values() {
            if !node.conflicts.is_empty() && conflict_examples < 3 {
                println!("     - {}: {} conflicts", 
                         node.path_before.display(), 
                         node.conflicts.len());
                for conflict in &node.conflicts {
                    println!("       └─ {:?}", conflict);
                }
                conflict_examples += 1;
            }
        }
        
        if plan.summary.conflicts as usize > conflict_examples {
            println!("     ... and {} more conflicts", 
                     plan.summary.conflicts as usize - conflict_examples);
        }
        has_issues = true;
    }
    
    // Check for warnings
    if plan.summary.warnings > 0 {
        println!("  ⚠️  {} operations have warnings", plan.summary.warnings);
        
        let mut warning_types = std::collections::HashMap::new();
        for node in plan.nodes.values() {
            for warning in &node.warnings {
                *warning_types.entry(format!("{:?}", warning)).or_insert(0) += 1;
            }
        }
        
        for (warning_type, count) in warning_types {
            println!("     - {}: {} operations", warning_type, count);
        }
        has_issues = true;
    }
    
    // Check for cross-volume operations
    if plan.summary.cross_volume > 0 {
        println!("  ℹ️  {} cross-volume operations (will be slower)", plan.summary.cross_volume);
    }
    
    // Check for long paths
    let mut long_paths = 0;
    for node in plan.nodes.values() {
        if node.path_after.to_string_lossy().len() > 260 {
            long_paths += 1;
        }
    }
    
    if long_paths > 0 {
        println!("  ⚠️  {} operations result in long paths (>260 chars)", long_paths);
        has_issues = true;
    }
    
    if !has_issues {
        println!("  ✅ No significant issues detected");
    }
}

fn provide_recommendations(plan: &MovePlan, simulation: &SimulationReport) {
    println!("\n💡 Recommendations:");
    
    let total_ops = plan.nodes.len();
    let success_rate = if total_ops > 0 {
        (simulation.success_estimate as f64 / total_ops as f64) * 100.0
    } else {
        100.0
    };
    
    if success_rate < 90.0 {
        println!("  ⚠️  Success rate is {:.1}% - consider resolving conflicts first", success_rate);
        println!("     └─ Use plan editing tools to resolve conflicts before execution");
    } else if success_rate < 100.0 {
        println!("  ✅ Success rate is {:.1}% - mostly ready for execution", success_rate);
    } else {
        println!("  ✅ All operations should succeed - plan looks good!");
    }
    
    if simulation.conflicts_remaining > 0 {
        println!("  🔧 To resolve conflicts:");
        println!("     └─ Edit move plan to rename conflicting destinations");
        println!("     └─ Set conflict policy to 'AutoRename' for automatic resolution");
        println!("     └─ Mark problematic operations as 'Skip' to exclude them");
    }
    
    if plan.summary.cross_volume > 0 {
        println!("  ⏱️  Cross-volume operations detected:");
        println!("     └─ These will copy then delete (slower than move)");
        println!("     └─ Ensure sufficient disk space on destination volumes");
    }
    
    if simulation.estimated_duration.as_secs() > 300 { // 5 minutes
        println!("  ⏱️  Estimated duration is long ({:?})", simulation.estimated_duration);
        println!("     └─ Consider running in batches for better control");
        println!("     └─ Ensure system won't sleep/hibernate during execution");
    }
    
    println!("\n🚀 Next Steps:");
    if simulation.conflicts_remaining == 0 {
        println!("  1. Review the plan summary above");
        println!("  2. Run: filemover apply --plan {}", 
                 "move_plan.json"); // This would be the actual plan file path
        println!("  3. Monitor progress and check logs");
    } else {
        println!("  1. Resolve {} conflicts in the plan", simulation.conflicts_remaining);
        println!("  2. Re-run dry-run to verify fixes");
        println!("  3. Execute when ready with: filemover apply --plan [plan-file]");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use filemover_types::{PlanNode, PlanNodeId, PlanSummary, Warning, Conflict};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_dry_run_command_with_missing_plan() {
        let config_manager = ConfigManager::new(None).unwrap();
        
        let result = dry_run_command(
            PathBuf::from("nonexistent_plan.json"),
            &config_manager
        ).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_load_move_plan() {
        let temp_dir = TempDir::new().unwrap();
        let plan_file = temp_dir.path().join("test_plan.json");
        
        // Create test plan
        let mut nodes = HashMap::new();
        let node_id = PlanNodeId::new();
        let node = PlanNode {
            id: node_id,
            is_dir: true,
            name_before: "test_folder".to_string(),
            path_before: PathBuf::from("C:\\Test\\test_folder"),
            name_after: "test_folder".to_string(),
            path_after: PathBuf::from("D:\\Archive\\test_folder"),
            kind: OpKind::Move,
            size_bytes: Some(1024),
            warnings: vec![Warning::LongPath],
            conflicts: vec![],
            children: vec![],
            rule_id: None,
        };
        nodes.insert(node_id, node);
        
        let plan = MovePlan {
            roots: vec![node_id],
            nodes,
            summary: PlanSummary {
                count_dirs: 1,
                count_files: 0,
                total_bytes: Some(1024),
                cross_volume: 1,
                conflicts: 0,
                warnings: 1,
            },
        };
        
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_file, json).unwrap();
        
        let loaded = load_move_plan(&plan_file).unwrap();
        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.summary.count_dirs, 1);
    }
    
    #[test]
    fn test_analyze_potential_issues() {
        let mut nodes = HashMap::new();
        let node_id = PlanNodeId::new();
        let node = PlanNode {
            id: node_id,
            is_dir: true,
            name_before: "test".to_string(),
            path_before: PathBuf::from("C:\\Test"),
            name_after: "test".to_string(),
            path_after: PathBuf::from("D:\\Archive\\very\\long\\path\\that\\exceeds\\the\\normal\\windows\\path\\limit\\of\\260\\characters\\and\\should\\trigger\\a\\warning\\about\\long\\paths\\which\\might\\cause\\issues\\on\\some\\systems\\that\\dont\\support\\long\\path\\names\\properly\\test"),
            kind: OpKind::Move,
            size_bytes: None,
            warnings: vec![Warning::LongPath],
            conflicts: vec![Conflict::NameExists { existing_path: PathBuf::from("D:\\Archive\\test") }],
            children: vec![],
            rule_id: None,
        };
        nodes.insert(node_id, node);
        
        let plan = MovePlan {
            roots: vec![node_id],
            nodes,
            summary: PlanSummary {
                count_dirs: 1,
                count_files: 0,
                total_bytes: None,
                cross_volume: 1,
                conflicts: 1,
                warnings: 1,
            },
        };
        
        // This should not panic and should identify issues
        analyze_potential_issues(&plan);
    }
}