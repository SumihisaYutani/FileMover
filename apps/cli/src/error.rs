use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Invalid command arguments: {message}")]
    InvalidArgs { message: String },

    #[error("File operation failed: {path}: {message}")]
    FileOperation { path: PathBuf, message: String },

    #[error("Profile '{profile}' not found")]
    ProfileNotFound { profile: String },

    #[error("Scan failed: {message}")]
    Scan { message: String },

    #[error("Plan generation failed: {message}")]
    Planning { message: String },

    #[error("Execution failed: {message}")]
    Execution { message: String },

    #[error("Undo operation failed: {message}")]
    Undo { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Core library error: {0}")]
    Core(#[from] filemover_types::FileMoverError),
}

impl CliError {
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn invalid_args<S: Into<String>>(message: S) -> Self {
        Self::InvalidArgs {
            message: message.into(),
        }
    }

    pub fn file_operation<S: Into<String>>(path: PathBuf, message: S) -> Self {
        Self::FileOperation {
            path,
            message: message.into(),
        }
    }

    pub fn profile_not_found<S: Into<String>>(profile: S) -> Self {
        Self::ProfileNotFound {
            profile: profile.into(),
        }
    }

    pub fn scan<S: Into<String>>(message: S) -> Self {
        Self::Scan {
            message: message.into(),
        }
    }

    pub fn planning<S: Into<String>>(message: S) -> Self {
        Self::Planning {
            message: message.into(),
        }
    }

    pub fn execution<S: Into<String>>(message: S) -> Self {
        Self::Execution {
            message: message.into(),
        }
    }

    pub fn undo<S: Into<String>>(message: S) -> Self {
        Self::Undo {
            message: message.into(),
        }
    }

    /// Get user-friendly error message for display
    pub fn user_message(&self) -> String {
        match self {
            Self::Config { message } => {
                format!("⚙️ Configuration Error: {}", message)
            }
            Self::InvalidArgs { message } => {
                format!("❌ Invalid Arguments: {}", message)
            }
            Self::FileOperation { path, message } => {
                format!("📁 File Error ({}): {}", path.display(), message)
            }
            Self::ProfileNotFound { profile } => {
                format!("📋 Profile '{}' not found. Use 'filemover config list' to see available profiles.", profile)
            }
            Self::Scan { message } => {
                format!("🔍 Scan Error: {}", message)
            }
            Self::Planning { message } => {
                format!("📝 Planning Error: {}", message)
            }
            Self::Execution { message } => {
                format!("🚀 Execution Error: {}", message)
            }
            Self::Undo { message } => {
                format!("🔄 Undo Error: {}", message)
            }
            Self::Io(e) => {
                format!("💾 File System Error: {}", e)
            }
            Self::Json(e) => {
                format!("📄 Data Format Error: {}", e)
            }
            Self::Core(e) => {
                format!("🔧 Internal Error: {}", e)
            }
        }
    }

    /// Get error code for programmatic handling
    pub fn error_code(&self) -> i32 {
        match self {
            Self::Config { .. } => 10,
            Self::InvalidArgs { .. } => 11,
            Self::FileOperation { .. } => 12,
            Self::ProfileNotFound { .. } => 13,
            Self::Scan { .. } => 20,
            Self::Planning { .. } => 21,
            Self::Execution { .. } => 22,
            Self::Undo { .. } => 23,
            Self::Io(_) => 30,
            Self::Json(_) => 31,
            Self::Core(_) => 40,
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Config { .. } => true,
            Self::InvalidArgs { .. } => true,
            Self::FileOperation { .. } => false, // File system errors are usually not recoverable
            Self::ProfileNotFound { .. } => true,
            Self::Scan { .. } => true,
            Self::Planning { .. } => true,
            Self::Execution { .. } => false, // Execution errors can be dangerous
            Self::Undo { .. } => false, // Undo errors are critical
            Self::Io(_) => false,
            Self::Json(_) => true, // Data format errors might be fixable
            Self::Core(_) => false, // Core errors indicate serious issues
        }
    }

    /// Get suggestions for resolving the error
    pub fn suggestions(&self) -> Vec<String> {
        match self {
            Self::Config { .. } => vec![
                "Check your configuration file syntax".to_string(),
                "Verify file paths exist and are accessible".to_string(),
                "Use 'filemover config show' to validate settings".to_string(),
            ],
            Self::InvalidArgs { .. } => vec![
                "Check command syntax with 'filemover --help'".to_string(),
                "Verify all required arguments are provided".to_string(),
            ],
            Self::FileOperation { .. } => vec![
                "Ensure the file or directory exists".to_string(),
                "Check file permissions".to_string(),
                "Verify disk space is available".to_string(),
            ],
            Self::ProfileNotFound { .. } => vec![
                "Use 'filemover config list' to see available profiles".to_string(),
                "Create the profile with 'filemover config create'".to_string(),
            ],
            Self::Scan { .. } => vec![
                "Verify root directories exist and are accessible".to_string(),
                "Check pattern syntax in rules".to_string(),
                "Try with '--verbose' for detailed logging".to_string(),
            ],
            Self::Planning { .. } => vec![
                "Verify scan results file is valid".to_string(),
                "Check rules configuration".to_string(),
                "Ensure destination paths are valid".to_string(),
            ],
            Self::Execution { .. } => vec![
                "Run dry-run first to check for issues".to_string(),
                "Ensure sufficient permissions".to_string(),
                "Check disk space on destination".to_string(),
                "Review the journal file for partial results".to_string(),
            ],
            Self::Undo { .. } => vec![
                "Verify journal file is valid and complete".to_string(),
                "Check that moved files still exist at destinations".to_string(),
                "Consider manual recovery if automatic undo fails".to_string(),
            ],
            Self::Io(_) => vec![
                "Check file permissions".to_string(),
                "Verify disk space".to_string(),
                "Ensure files are not locked by other applications".to_string(),
            ],
            Self::Json(_) => vec![
                "Validate JSON file syntax".to_string(),
                "Check for corrupted files".to_string(),
                "Recreate the file if necessary".to_string(),
            ],
            Self::Core(_) => vec![
                "Try restarting the application".to_string(),
                "Check system resources".to_string(),
                "Report this issue if it persists".to_string(),
            ],
        }
    }
}

// Helper function to display error with suggestions
pub fn display_error_with_help(error: &CliError) {
    eprintln!("{}", error.user_message());
    
    let suggestions = error.suggestions();
    if !suggestions.is_empty() {
        eprintln!("\n💡 Suggestions:");
        for (i, suggestion) in suggestions.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, suggestion);
        }
    }
    
    if error.is_recoverable() {
        eprintln!("\n🔄 This error may be recoverable. Please try the suggestions above.");
    } else {
        eprintln!("\n⚠️  This is a critical error that requires manual intervention.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = CliError::config("Test configuration error");
        assert!(matches!(error, CliError::Config { .. }));
    }

    #[test]
    fn test_user_message() {
        let error = CliError::scan("Test scan error");
        let message = error.user_message();
        assert!(message.contains("🔍"));
        assert!(message.contains("Test scan error"));
    }

    #[test]
    fn test_error_code() {
        assert_eq!(CliError::config("test").error_code(), 10);
        assert_eq!(CliError::scan("test").error_code(), 20);
        assert_eq!(CliError::execution("test").error_code(), 22);
    }

    #[test]
    fn test_is_recoverable() {
        assert!(CliError::config("test").is_recoverable());
        assert!(CliError::planning("test").is_recoverable());
        assert!(!CliError::execution("test").is_recoverable());
        assert!(!CliError::undo("test").is_recoverable());
    }

    #[test]
    fn test_suggestions() {
        let error = CliError::scan("test");
        let suggestions = error.suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("verbose")));
    }

    #[test]
    fn test_from_conversions() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let cli_error = CliError::from(io_error);
        assert!(matches!(cli_error, CliError::Io(_)));
    }
}