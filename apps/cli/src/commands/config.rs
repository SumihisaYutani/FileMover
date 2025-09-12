use anyhow::{Result, Context};
use tracing::info;

use filemover_types::{Config, Rule, PatternSpec, ScanOptions};
use crate::{ConfigAction, config_manager::ConfigManager};

pub async fn config_command(
    action: ConfigAction,
    config_manager: &ConfigManager,
) -> Result<()> {
    match action {
        ConfigAction::List => list_profiles(config_manager).await,
        ConfigAction::Show { profile } => show_profile(profile, config_manager).await,
        ConfigAction::Create { profile, from } => create_profile(profile, from, config_manager).await,
        ConfigAction::Delete { profile } => delete_profile(profile, config_manager).await,
    }
}

async fn list_profiles(config_manager: &ConfigManager) -> Result<()> {
    info!("Listing available profiles");
    
    let profiles = config_manager.list_profiles()
        .context("Failed to list profiles")?;
    
    if profiles.is_empty() {
        println!("No configuration profiles found.");
        println!("Create a new profile with: filemover config create <name>");
        return Ok(());
    }
    
    println!("📋 Available Configuration Profiles:");
    for (i, profile) in profiles.iter().enumerate() {
        let is_default = profile == "default";
        let marker = if is_default { " (default)" } else { "" };
        println!("  {}. {}{}", i + 1, profile, marker);
    }
    
    println!("\nUse 'filemover config show <profile>' to view profile details.");
    
    Ok(())
}

async fn show_profile(profile_name: String, config_manager: &ConfigManager) -> Result<()> {
    info!("Showing profile: {}", profile_name);
    
    let config = config_manager.load_config(Some(&profile_name))
        .with_context(|| format!("Failed to load profile '{}'", profile_name))?;
    
    println!("📄 Profile: {}", profile_name);
    println!("=" .repeat(50));
    
    // Show roots
    println!("\n📁 Scan Roots ({}):", config.roots.len());
    if config.roots.is_empty() {
        println!("  (none configured)");
    } else {
        for (i, root) in config.roots.iter().enumerate() {
            println!("  {}. {}", i + 1, root.display());
        }
    }
    
    // Show rules
    println!("\n📝 Rules ({}):", config.rules.len());
    if config.rules.is_empty() {
        println!("  (no rules configured)");
    } else {
        for (i, rule) in config.rules.iter().enumerate() {
            let enabled_marker = if rule.enabled { "✅" } else { "❌" };
            let exclude_marker = if rule.pattern.is_exclude { " (exclude)" } else { "" };
            
            println!("  {}. {} [Priority: {}] {}{}", 
                     i + 1, 
                     enabled_marker, 
                     rule.priority,
                     format_pattern(&rule.pattern),
                     exclude_marker
            );
            println!("     → {} / {}", 
                     rule.dest_root.display(), 
                     rule.template);
            if let Some(label) = &rule.label {
                println!("     Label: {}", label);
            }
        }
    }
    
    // Show scan options
    println!("\n⚙️ Scan Options:");
    println!("  Follow junctions: {}", 
             if config.options.follow_junctions { "Yes" } else { "No" });
    println!("  System protections: {}", 
             if config.options.system_protections { "Enabled" } else { "Disabled" });
    println!("  Max depth: {}", 
             config.options.max_depth.map(|d| d.to_string()).unwrap_or("Unlimited".to_string()));
    
    if !config.options.excluded_paths.is_empty() {
        println!("  Excluded paths:");
        for path in &config.options.excluded_paths {
            println!("    - {}", path.display());
        }
    }
    
    // Show normalization options
    println!("\n🔤 Text Normalization:");
    println!("  Unicode normalization: {}", 
             if config.options.normalization.normalize_unicode { "Enabled" } else { "Disabled" });
    println!("  Width normalization: {}", 
             if config.options.normalization.normalize_width { "Enabled" } else { "Disabled" });
    println!("  Strip diacritics: {}", 
             if config.options.normalization.strip_diacritics { "Enabled" } else { "Disabled" });
    println!("  Case normalization: {}", 
             if config.options.normalization.normalize_case { "Enabled" } else { "Disabled" });
    
    Ok(())
}

fn format_pattern(pattern: &PatternSpec) -> String {
    let kind_str = match pattern.kind {
        filemover_types::PatternKind::Glob => "Glob",
        filemover_types::PatternKind::Regex => "Regex", 
        filemover_types::PatternKind::Contains => "Contains",
    };
    
    let case_str = if pattern.case_insensitive { " (case-insensitive)" } else { "" };
    
    format!("{}: \"{}\"{}",  kind_str, pattern.value, case_str)
}

async fn create_profile(
    profile_name: String,
    from_profile: Option<String>,
    config_manager: &ConfigManager,
) -> Result<()> {
    info!("Creating profile: {}", profile_name);
    
    // Check if profile already exists
    if config_manager.profile_exists(&profile_name)? {
        anyhow::bail!("Profile '{}' already exists", profile_name);
    }
    
    // Load base configuration
    let base_config = if let Some(from) = from_profile {
        println!("📋 Copying from profile: {}", from);
        config_manager.load_config(Some(&from))
            .with_context(|| format!("Failed to load base profile '{}'", from))?
    } else {
        println!("📋 Creating new profile with default settings");
        Config::default()
    };
    
    // Save new profile
    config_manager.save_config(&profile_name, &base_config)
        .with_context(|| format!("Failed to save profile '{}'", profile_name))?;
    
    println!("✅ Profile '{}' created successfully", profile_name);
    println!("Edit the configuration file to customize settings:");
    println!("  {}", config_manager.get_profile_path(&profile_name).display());
    
    Ok(())
}

async fn delete_profile(profile_name: String, config_manager: &ConfigManager) -> Result<()> {
    info!("Deleting profile: {}", profile_name);
    
    if profile_name == "default" {
        anyhow::bail!("Cannot delete the default profile");
    }
    
    if !config_manager.profile_exists(&profile_name)? {
        anyhow::bail!("Profile '{}' does not exist", profile_name);
    }
    
    // Confirm deletion
    print!("⚠️  Delete profile '{}'? This cannot be undone. (y/N): ", profile_name);
    std::io::Write::flush(&mut std::io::stdout())?;
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    let input = input.trim().to_lowercase();
    if input != "y" && input != "yes" {
        println!("Deletion cancelled.");
        return Ok(());
    }
    
    // Delete profile
    config_manager.delete_profile(&profile_name)
        .with_context(|| format!("Failed to delete profile '{}'", profile_name))?;
    
    println!("✅ Profile '{}' deleted successfully", profile_name);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_list_empty_profiles() {
        let temp_dir = TempDir::new().unwrap();
        let config_manager = ConfigManager::new(Some(temp_dir.path().join("config.json"))).unwrap();
        
        // Should not panic with empty profiles
        let result = list_profiles(&config_manager).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_show_default_profile() {
        let temp_dir = TempDir::new().unwrap();
        let config_manager = ConfigManager::new(Some(temp_dir.path().join("config.json"))).unwrap();
        
        // Create a default config
        let config = Config::default();
        config_manager.save_config("default", &config).unwrap();
        
        let result = show_profile("default".to_string(), &config_manager).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_create_profile() {
        let temp_dir = TempDir::new().unwrap();
        let config_manager = ConfigManager::new(Some(temp_dir.path().join("config.json"))).unwrap();
        
        let result = create_profile("test_profile".to_string(), None, &config_manager).await;
        assert!(result.is_ok());
        
        // Verify profile was created
        assert!(config_manager.profile_exists("test_profile").unwrap());
    }
    
    #[tokio::test]
    async fn test_create_duplicate_profile() {
        let temp_dir = TempDir::new().unwrap();
        let config_manager = ConfigManager::new(Some(temp_dir.path().join("config.json"))).unwrap();
        
        // Create profile first time
        let result1 = create_profile("test_profile".to_string(), None, &config_manager).await;
        assert!(result1.is_ok());
        
        // Try to create same profile again
        let result2 = create_profile("test_profile".to_string(), None, &config_manager).await;
        assert!(result2.is_err());
    }
    
    #[test]
    fn test_format_pattern() {
        let pattern = PatternSpec::new_glob("test*").case_sensitive();
        let formatted = format_pattern(&pattern);
        assert!(formatted.contains("Glob"));
        assert!(formatted.contains("test*"));
        assert!(!formatted.contains("case-insensitive"));
        
        let pattern2 = PatternSpec::new_regex(".*\\.jpg$");
        let formatted2 = format_pattern(&pattern2);
        assert!(formatted2.contains("Regex"));
        assert!(formatted2.contains("case-insensitive"));
    }
}