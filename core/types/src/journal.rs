use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::plan::OpKind;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ResultKind {
    Ok,
    Skip,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub when_utc: DateTime<Utc>,
    pub source: PathBuf,
    pub dest: PathBuf,
    pub op: OpKind,
    pub result: ResultKind,
    pub message: Option<String>,
}

impl JournalEntry {
    pub fn new(source: PathBuf, dest: PathBuf, op: OpKind) -> Self {
        Self {
            when_utc: Utc::now(),
            source,
            dest,
            op,
            result: ResultKind::Ok,
            message: None,
        }
    }

    pub fn with_result(mut self, result: ResultKind) -> Self {
        self.result = result;
        self
    }

    pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn failed<S: Into<String>>(mut self, message: S) -> Self {
        self.result = ResultKind::Failed;
        self.message = Some(message.into());
        self
    }

    pub fn skipped<S: Into<String>>(mut self, message: S) -> Self {
        self.result = ResultKind::Skip;
        self.message = Some(message.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UndoResult {
    pub restored_count: u64,
    pub failed_count: u64,
    pub total_duration: std::time::Duration,
    pub failed_restores: Vec<FailedRestore>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailedRestore {
    pub original_source: PathBuf,
    pub original_dest: PathBuf,
    pub error: String,
}