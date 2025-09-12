use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::rule::{Rule, ConflictPolicy};
use crate::pattern::NormalizationOptions;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub roots: Vec<PathBuf>,
    pub rules: Vec<Rule>,
    pub options: ScanOptions,
    pub profiles: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roots: vec![],
            rules: vec![],
            options: ScanOptions::default(),
            profiles: vec!["Default".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanOptions {
    pub normalization: NormalizationOptions,
    pub follow_junctions: bool,
    pub system_protections: bool,
    pub max_depth: Option<u32>,
    pub excluded_paths: Vec<PathBuf>,
    pub parallel_threads: Option<usize>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            normalization: NormalizationOptions::default(),
            follow_junctions: false,
            system_protections: true,
            max_depth: None,
            excluded_paths: Self::default_excluded_paths(),
            parallel_threads: None,
        }
    }
}

impl ScanOptions {
    fn default_excluded_paths() -> Vec<PathBuf> {
        vec![
            PathBuf::from("C:\\Windows"),
            PathBuf::from("C:\\Program Files"),
            PathBuf::from("C:\\Program Files (x86)"),
            PathBuf::from("$Recycle.Bin"),
            PathBuf::from("System Volume Information"),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanOptions {
    pub default_conflict_policy: ConflictPolicy,
    pub preserve_acl: bool,
    pub preserve_timestamps: bool,
    pub enable_cross_volume: bool,
    pub dry_run_only: bool,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            default_conflict_policy: ConflictPolicy::AutoRename,
            preserve_acl: true,
            preserve_timestamps: true,
            enable_cross_volume: true,
            dry_run_only: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub current_item: Option<String>,
    pub completed_ops: u64,
    pub total_ops: u64,
    pub bytes_processed: u64,
    pub total_bytes: Option<u64>,
    pub current_speed: Option<u64>, // bytes/sec
    pub eta: Option<std::time::Duration>,
}

impl Progress {
    pub fn new(total_ops: u64, total_bytes: Option<u64>) -> Self {
        Self {
            current_item: None,
            completed_ops: 0,
            total_ops,
            bytes_processed: 0,
            total_bytes,
            current_speed: None,
            eta: None,
        }
    }

    pub fn percentage(&self) -> f64 {
        if self.total_ops == 0 {
            100.0
        } else {
            (self.completed_ops as f64 / self.total_ops as f64) * 100.0
        }
    }
}