use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use tracing::{debug, info};

use filemover_types::Config;

pub struct ConfigManager {
    config_dir: PathBuf,
    default_config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(config_file: Option<PathBuf>) -> Result<Self> {
        let (config_dir, default_config_path) = if let Some(path) = config_file {
            let dir = path.parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid config file path"))?
                .to_path_buf();
            (dir, path)
        } else {
            Self::default_config_paths()?
        };

        // Ensure config directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create configuration directory")?;
            info!("Created configuration directory: {}", config_dir.display());
        }

        Ok(Self {
            config_dir,
            default_config_path,
        })
    }

    fn default_config_paths() -> Result<(PathBuf, PathBuf)> {
        #[cfg(windows)]
        {
            use std::env;
            let app_data = env::var("APPDATA")
                .or_else(|_| env::var("USERPROFILE"))
                .context("Failed to determine user configuration directory")?;
            
            let config_dir = PathBuf::from(app_data).join("FileMover");
            let default_config = config_dir.join("config.json");
            
            Ok((config_dir, default_config))
        }
        
        #[cfg(not(windows))]
        {
            use std::env;
            let home = env::var("HOME")
                .context("Failed to determine home directory")?;
            
            let config_dir = PathBuf::from(home).join(".config").join("filemover");
            let default_config = config_dir.join("config.json");
            
            Ok((config_dir, default_config))
        }
    }

    pub fn load_config(&self, profile: Option<&str>) -> Result<Config> {
        let config_path = match profile {
            Some(name) => self.get_profile_path(name),
            None => self.default_config_path.clone(),
        };

        debug!("Loading configuration from: {}", config_path.display());

        if !config_path.exists() {
            if profile.is_some() {
                anyhow::bail!("Profile '{}' not found at: {}", profile.unwrap(), config_path.display());
            } else {
                // Create default config if it doesn't exist
                let default_config = Config::default();
                self.save_config_to_path(&config_path, &default_config)?;
                info!("Created default configuration at: {}", config_path.display());
                return Ok(default_config);
            }
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let config: Config = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

        Ok(config)
    }

    pub fn save_config(&self, profile: &str, config: &Config) -> Result<()> {
        let config_path = self.get_profile_path(profile);
        self.save_config_to_path(&config_path, config)
    }

    fn save_config_to_path(&self, path: &Path, config: &Config) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let json = serde_json::to_string_pretty(config)
            .context("Failed to serialize configuration")?;

        fs::write(path, json)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        debug!("Configuration saved to: {}", path.display());
        Ok(())
    }

    pub fn get_profile_path(&self, profile: &str) -> PathBuf {
        if profile == "default" {
            self.default_config_path.clone()
        } else {
            self.config_dir.join(format!("{}.json", profile))
        }
    }

    pub fn list_profiles(&self) -> Result<Vec<String>> {
        let mut profiles = Vec::new();

        // Always include default if it exists
        if self.default_config_path.exists() {
            profiles.push("default".to_string());
        }

        // Scan for other profile files
        if self.config_dir.exists() {
            let entries = fs::read_dir(&self.config_dir)
                .context("Failed to read configuration directory")?;

            for entry in entries {
                let entry = entry.context("Failed to read directory entry")?;
                let path = entry.path();

                if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem != "config" { // Skip default config
                            profiles.push(stem.to_string());
                        }
                    }
                }
            }
        }

        profiles.sort();
        Ok(profiles)
    }

    pub fn profile_exists(&self, profile: &str) -> Result<bool> {
        let path = self.get_profile_path(profile);
        Ok(path.exists())
    }

    pub fn delete_profile(&self, profile: &str) -> Result<()> {
        if profile == "default" {
            anyhow::bail!("Cannot delete default profile");
        }

        let path = self.get_profile_path(profile);
        
        if !path.exists() {
            anyhow::bail!("Profile '{}' does not exist", profile);
        }

        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete profile file: {}", path.display()))?;

        info!("Deleted profile: {}", profile);
        Ok(())
    }

    pub fn get_config_dir(&self) -> &Path {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use filemover_types::{Rule, PatternSpec};

    #[test]
    fn test_config_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test_config.json");
        
        let manager = ConfigManager::new(Some(config_file)).unwrap();
        assert!(manager.config_dir.exists());
    }

    #[test]
    fn test_default_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.json");
        
        let manager = ConfigManager::new(Some(config_file)).unwrap();
        
        // Loading non-existent default config should create one
        let config = manager.load_config(None).unwrap();
        assert_eq!(config.roots.len(), 0); // Default config has empty roots
        
        // Config file should now exist
        assert!(manager.default_config_path.exists());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.json");
        
        let manager = ConfigManager::new(Some(config_file)).unwrap();
        
        // Create test config
        let mut config = Config::default();
        config.roots.push(PathBuf::from("C:\\Test"));
        config.rules.push(Rule::new(
            PatternSpec::new_glob("test*"),
            PathBuf::from("D:\\Archive"),
            "{name}".to_string(),
        ));
        
        // Save config
        manager.save_config("test_profile", &config).unwrap();
        
        // Load config back
        let loaded_config = manager.load_config(Some("test_profile")).unwrap();
        
        assert_eq!(loaded_config.roots.len(), 1);
        assert_eq!(loaded_config.rules.len(), 1);
        assert_eq!(loaded_config.roots[0], PathBuf::from("C:\\Test"));
    }

    #[test]
    fn test_list_profiles() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.json");
        
        let manager = ConfigManager::new(Some(config_file)).unwrap();
        
        // Initially no profiles
        let profiles = manager.list_profiles().unwrap();
        assert_eq!(profiles.len(), 0);
        
        // Create default config
        manager.save_config("default", &Config::default()).unwrap();
        
        // Create test profile
        manager.save_config("test", &Config::default()).unwrap();
        
        let profiles = manager.list_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains(&"default".to_string()));
        assert!(profiles.contains(&"test".to_string()));
    }

    #[test]
    fn test_profile_exists() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.json");
        
        let manager = ConfigManager::new(Some(config_file)).unwrap();
        
        assert!(!manager.profile_exists("nonexistent").unwrap());
        
        manager.save_config("test", &Config::default()).unwrap();
        assert!(manager.profile_exists("test").unwrap());
    }

    #[test]
    fn test_delete_profile() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.json");
        
        let manager = ConfigManager::new(Some(config_file)).unwrap();
        
        // Create test profile
        manager.save_config("test", &Config::default()).unwrap();
        assert!(manager.profile_exists("test").unwrap());
        
        // Delete profile
        manager.delete_profile("test").unwrap();
        assert!(!manager.profile_exists("test").unwrap());
    }

    #[test]
    fn test_cannot_delete_default_profile() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.json");
        
        let manager = ConfigManager::new(Some(config_file)).unwrap();
        
        let result = manager.delete_profile("default");
        assert!(result.is_err());
    }
}