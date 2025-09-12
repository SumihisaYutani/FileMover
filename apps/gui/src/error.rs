use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum GuiError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Scan error: {message}")]
    Scan { message: String },

    #[error("Planning error: {message}")]
    Planning { message: String },

    #[error("Execution error: {message}")]
    Execution { message: String },

    #[error("Session not found: {id}")]
    SessionNotFound { id: String },

    #[error("Invalid operation: {message}")]
    InvalidOperation { message: String },

    #[error("File system error: {message}")]
    FileSystem { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Core library error: {0}")]
    Core(String),
}

impl From<filemover_types::FileMoverError> for GuiError {
    fn from(error: filemover_types::FileMoverError) -> Self {
        GuiError::Core(error.to_string())
    }
}

impl From<serde_json::Error> for GuiError {
    fn from(error: serde_json::Error) -> Self {
        GuiError::Serialization {
            message: error.to_string(),
        }
    }
}

impl From<std::io::Error> for GuiError {
    fn from(error: std::io::Error) -> Self {
        GuiError::FileSystem {
            message: error.to_string(),
        }
    }
}

// Result type for Tauri commands
pub type GuiResult<T> = Result<T, GuiError>;

// Helper to convert GuiError to a format Tauri can serialize
impl GuiError {
    pub fn to_frontend_error(&self) -> FrontendError {
        FrontendError {
            code: self.error_code(),
            message: self.to_string(),
            details: self.error_details(),
        }
    }

    fn error_code(&self) -> String {
        match self {
            GuiError::Config { .. } => "CONFIG_ERROR".to_string(),
            GuiError::Scan { .. } => "SCAN_ERROR".to_string(),
            GuiError::Planning { .. } => "PLANNING_ERROR".to_string(),
            GuiError::Execution { .. } => "EXECUTION_ERROR".to_string(),
            GuiError::SessionNotFound { .. } => "SESSION_NOT_FOUND".to_string(),
            GuiError::InvalidOperation { .. } => "INVALID_OPERATION".to_string(),
            GuiError::FileSystem { .. } => "FILE_SYSTEM_ERROR".to_string(),
            GuiError::Serialization { .. } => "SERIALIZATION_ERROR".to_string(),
            GuiError::Core(_) => "CORE_ERROR".to_string(),
        }
    }

    fn error_details(&self) -> Option<String> {
        match self {
            GuiError::SessionNotFound { id } => Some(format!("Session ID: {}", id)),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

// Macro to create GuiError variants easily
macro_rules! gui_error {
    (config, $msg:expr) => {
        GuiError::Config { message: $msg.to_string() }
    };
    (scan, $msg:expr) => {
        GuiError::Scan { message: $msg.to_string() }
    };
    (planning, $msg:expr) => {
        GuiError::Planning { message: $msg.to_string() }
    };
    (execution, $msg:expr) => {
        GuiError::Execution { message: $msg.to_string() }
    };
    (invalid_op, $msg:expr) => {
        GuiError::InvalidOperation { message: $msg.to_string() }
    };
    (session_not_found, $id:expr) => {
        GuiError::SessionNotFound { id: $id.to_string() }
    };
}

pub(crate) use gui_error;