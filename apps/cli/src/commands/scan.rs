use std::path::PathBuf;
use anyhow::{Result, Context};
use tracing::{info, debug};
use indicatif::{ProgressBar, ProgressStyle};

use filemover_types::{ScanOptions, Rule, FolderHit};
use filemover_scanner::FolderScanner;
use crate::config_manager::ConfigManager;

pub async fn scan_command(
    roots: Vec<PathBuf>,
    output_file: Option<PathBuf>,
    profile: Option<String>,
    config_manager: &ConfigManager,
) -> Result<()> {
    info!("Starting folder scan");
    
    // Load configuration
    let config = config_manager.load_config(profile.as_deref())?;
    
    // Use provided roots or fall back to config
    let scan_roots = if roots.is_empty() {
        if config.roots.is_empty() {
            anyhow::bail!("No root directories specified. Use --roots or configure in profile.");
        }
        config.roots.clone()
    } else {
        roots
    };
    
    info!("Scanning {} root directories", scan_roots.len());
    for root in &scan_roots {
        info!("  - {}", root.display());
    }
    
    // Validate roots
    for root in &scan_roots {
        if !root.exists() {
            anyhow::bail!("Root directory does not exist: {}", root.display());
        }
        if !root.is_dir() {
            anyhow::bail!("Root path is not a directory: {}", root.display());
        }
    }
    
    // Create progress bar
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    progress.set_message("Initializing scanner...");
    
    // Initialize scanner
    let scanner = FolderScanner::new(config.rules.clone(), config.options.clone())
        .context("Failed to initialize folder scanner")?;
    
    progress.set_message("Scanning directories...");
    
    // Perform scan
    let scan_results = scanner.scan_roots(&scan_roots)
        .context("Failed to scan directories")?;
    
    progress.finish_with_message("Scan completed");
    
    // Display results summary
    print_scan_summary(&scan_results);
    
    // Save results
    let output_path = output_file.unwrap_or_else(|| {
        PathBuf::from(format!(
            "scan_results_{}.json",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
    });
    
    save_scan_results(&scan_results, &output_path)
        .context("Failed to save scan results")?;
    
    info!("Scan results saved to: {}", output_path.display());
    
    Ok(())
}

fn print_scan_summary(results: &[FolderHit]) {
    println!("\n=== Scan Results ===");
    println!("Total folders found: {}", results.len());
    
    if results.is_empty() {
        println!("No matching folders found.");
        return;
    }
    
    // Group by matched rule
    let mut rule_counts = std::collections::HashMap::new();
    let mut warning_counts = std::collections::HashMap::new();
    
    for hit in results {
        if let Some(rule_id) = &hit.matched_rule {
            *rule_counts.entry(*rule_id).or_insert(0) += 1;
        }
        
        for warning in &hit.warnings {
            *warning_counts.entry(format!("{:?}", warning)).or_insert(0) += 1;
        }
    }
    
    println!("\nMatched by rules:");
    for (rule_id, count) in rule_counts {
        println!("  Rule {}: {} folders", rule_id, count);
    }
    
    if !warning_counts.is_empty() {
        println!("\nWarnings:");
        for (warning, count) in warning_counts {
            println!("  {}: {} folders", warning, count);
        }
    }
    
    // Show first few matches
    println!("\nFirst 10 matches:");
    for (i, hit) in results.iter().take(10).enumerate() {
        println!("  {}. {} -> {:?}", 
                 i + 1, 
                 hit.path.display(),
                 hit.dest_preview.as_ref().map(|p| p.display().to_string()).unwrap_or("(no preview)".to_string())
        );
    }
    
    if results.len() > 10 {
        println!("  ... and {} more", results.len() - 10);
    }
}

fn save_scan_results(results: &[FolderHit], output_path: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(results)
        .context("Failed to serialize scan results")?;
    
    std::fs::write(output_path, json)
        .context("Failed to write scan results file")?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use filemover_types::{Config, PatternSpec};

    #[tokio::test]
    async fn test_scan_command_with_empty_roots() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        // Create config with empty roots
        let config = Config::default();
        let config_json = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&config_path, config_json).unwrap();
        
        let config_manager = ConfigManager::new(Some(config_path)).unwrap();
        
        // Should fail with empty roots
        let result = scan_command(vec![], None, None, &config_manager).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_scan_command_with_nonexistent_root() {
        let temp_dir = TempDir::new().unwrap();
        let config_manager = ConfigManager::new(None).unwrap();
        
        let nonexistent = PathBuf::from("/nonexistent/path");
        let result = scan_command(vec![nonexistent], None, None, &config_manager).await;
        
        assert!(result.is_err());
    }
}