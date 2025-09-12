use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::pattern::PatternSpec;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConflictPolicy {
    AutoRename,
    Skip,
    Overwrite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: Uuid,
    pub enabled: bool,
    pub pattern: PatternSpec,
    pub dest_root: PathBuf,
    pub template: String,
    pub policy: ConflictPolicy,
    pub label: Option<String>,
    pub priority: u32,
}

impl Rule {
    pub fn new(
        pattern: PatternSpec,
        dest_root: PathBuf,
        template: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            pattern,
            dest_root,
            template,
            policy: ConflictPolicy::AutoRename,
            label: None,
            priority: 0,
        }
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_policy(mut self, policy: ConflictPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FolderHit {
    pub path: PathBuf,
    pub name: String,
    pub matched_rule: Option<Uuid>,
    pub dest_preview: Option<PathBuf>,
    pub warnings: Vec<Warning>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Warning {
    LongPath,
    AclDiffers,
    Offline,
    AccessDenied,
    Junction,
    CrossVolume,
}