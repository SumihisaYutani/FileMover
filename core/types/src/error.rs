use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileMoverError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Pattern error: {message}")]
    Pattern { message: String },

    #[error("Scan error at path {path}: {message}")]
    Scan { path: PathBuf, message: String },

    #[error("Plan validation error: {message}")]
    PlanValidation { message: String },

    #[error("Execution error at {path}: {message}")]
    Execution { path: PathBuf, message: String },

    #[error("Undo error: {message}")]
    Undo { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Long path not supported: {path}")]
    LongPathNotSupported { path: PathBuf },

    #[error("OneDrive offline: {path}")]
    OneDriveOffline { path: PathBuf },

    #[error("Insufficient disk space: {path}")]
    InsufficientSpace { path: PathBuf },

    #[error("Invalid node ID: {0}")]
    InvalidNodeId(String),
}

