use std::collections::HashMap;
use std::path::PathBuf;
use std::fmt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::rule::{ConflictPolicy, Warning};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanNodeId(pub Uuid);

impl PlanNodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for PlanNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OpKind {
    Move,
    CopyDelete,
    Rename,
    Skip,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanNode {
    pub id: PlanNodeId,
    pub is_dir: bool,
    pub name_before: String,
    pub path_before: PathBuf,
    pub name_after: String,
    pub path_after: PathBuf,
    pub kind: OpKind,
    pub size_bytes: Option<u64>,
    pub warnings: Vec<Warning>,
    pub conflicts: Vec<Conflict>,
    pub children: Vec<PlanNodeId>,
    pub rule_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Conflict {
    NameExists { existing_path: PathBuf },
    CycleDetected,
    DestInsideSource,
    NoSpace { required: u64, available: u64 },
    Permission { required: Permission },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    Administrator,
    FileSystemWrite,
    NetworkAccess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovePlan {
    pub roots: Vec<PlanNodeId>,
    pub nodes: HashMap<PlanNodeId, PlanNode>,
    pub summary: PlanSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanSummary {
    pub count_dirs: u64,
    pub count_files: u64,
    pub total_bytes: Option<u64>,
    pub cross_volume: u64,
    pub conflicts: u64,
    pub warnings: u64,
}

impl Default for PlanSummary {
    fn default() -> Self {
        Self {
            count_dirs: 0,
            count_files: 0,
            total_bytes: None,
            cross_volume: 0,
            conflicts: 0,
            warnings: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeChange {
    SetSkip(PlanNodeId, bool),
    SetConflictPolicy(PlanNodeId, ConflictPolicy),
    RenameNode(PlanNodeId, String),
    ExcludeNode(PlanNodeId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationDelta {
    pub affected_nodes: Vec<PlanNodeId>,
    pub new_conflicts: Vec<Conflict>,
    pub resolved_conflicts: Vec<Conflict>,
    pub summary_diff: PlanSummaryDiff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanSummaryDiff {
    pub count_dirs_delta: i64,
    pub count_files_delta: i64,
    pub total_bytes_delta: Option<i64>,
    pub cross_volume_delta: i64,
    pub conflicts_delta: i64,
    pub warnings_delta: i64,
}