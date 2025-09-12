use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use filemover_types::{Config, FolderHit, MovePlan, Progress};
use filemover_scanner::FolderScanner;
use filemover_planner::MovePlanner;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub id: Uuid,
    pub roots: Vec<PathBuf>,
    pub status: SessionStatus,
    pub progress: Option<Progress>,
    pub results: Option<Vec<FolderHit>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSession {
    pub id: Uuid,
    pub scan_id: Option<Uuid>,
    pub status: SessionStatus,
    pub plan: Option<MovePlan>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSession {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub status: SessionStatus,
    pub progress: Option<Progress>,
    pub journal_path: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Created,
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub current_profile: Arc<Mutex<String>>,
    pub scan_sessions: Arc<Mutex<HashMap<Uuid, ScanSession>>>,
    pub plan_sessions: Arc<Mutex<HashMap<Uuid, PlanSession>>>,
    pub execution_sessions: Arc<Mutex<HashMap<Uuid, ExecutionSession>>>,
    pub scanner: Arc<Mutex<Option<FolderScanner>>>,
    pub planner: Arc<Mutex<MovePlanner>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(Config::default())),
            current_profile: Arc::new(Mutex::new("default".to_string())),
            scan_sessions: Arc::new(Mutex::new(HashMap::new())),
            plan_sessions: Arc::new(Mutex::new(HashMap::new())),
            execution_sessions: Arc::new(Mutex::new(HashMap::new())),
            scanner: Arc::new(Mutex::new(None)),
            planner: Arc::new(Mutex::new(MovePlanner::new())),
        }
    }

    pub fn create_scan_session(&self, roots: Vec<PathBuf>) -> Uuid {
        let id = Uuid::new_v4();
        let session = ScanSession {
            id,
            roots,
            status: SessionStatus::Created,
            progress: None,
            results: None,
            error: None,
        };

        if let Ok(mut sessions) = self.scan_sessions.lock() {
            sessions.insert(id, session);
        }

        id
    }

    pub fn create_plan_session(&self, scan_id: Option<Uuid>) -> Uuid {
        let id = Uuid::new_v4();
        let session = PlanSession {
            id,
            scan_id,
            status: SessionStatus::Created,
            plan: None,
            error: None,
        };

        if let Ok(mut sessions) = self.plan_sessions.lock() {
            sessions.insert(id, session);
        }

        id
    }

    pub fn create_execution_session(&self, plan_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        let session = ExecutionSession {
            id,
            plan_id,
            status: SessionStatus::Created,
            progress: None,
            journal_path: None,
            error: None,
        };

        if let Ok(mut sessions) = self.execution_sessions.lock() {
            sessions.insert(id, session);
        }

        id
    }

    pub fn update_scan_session<F>(&self, id: Uuid, updater: F) 
    where
        F: FnOnce(&mut ScanSession),
    {
        if let Ok(mut sessions) = self.scan_sessions.lock() {
            if let Some(session) = sessions.get_mut(&id) {
                updater(session);
            }
        }
    }

    pub fn update_plan_session<F>(&self, id: Uuid, updater: F)
    where
        F: FnOnce(&mut PlanSession),
    {
        if let Ok(mut sessions) = self.plan_sessions.lock() {
            if let Some(session) = sessions.get_mut(&id) {
                updater(session);
            }
        }
    }

    pub fn update_execution_session<F>(&self, id: Uuid, updater: F)
    where
        F: FnOnce(&mut ExecutionSession),
    {
        if let Ok(mut sessions) = self.execution_sessions.lock() {
            if let Some(session) = sessions.get_mut(&id) {
                updater(session);
            }
        }
    }

    pub fn get_scan_session(&self, id: Uuid) -> Option<ScanSession> {
        if let Ok(sessions) = self.scan_sessions.lock() {
            sessions.get(&id).cloned()
        } else {
            None
        }
    }

    pub fn get_plan_session(&self, id: Uuid) -> Option<PlanSession> {
        if let Ok(sessions) = self.plan_sessions.lock() {
            sessions.get(&id).cloned()
        } else {
            None
        }
    }

    pub fn get_execution_session(&self, id: Uuid) -> Option<ExecutionSession> {
        if let Ok(sessions) = self.execution_sessions.lock() {
            sessions.get(&id).cloned()
        } else {
            None
        }
    }

    pub fn cleanup_old_sessions(&self) {
        // Clean up sessions older than 1 hour
        // Implementation would check timestamps and remove old sessions
        // For now, just a placeholder
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}