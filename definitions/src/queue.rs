use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueState {
    pub paused: bool,
    pub stopped: bool,
    pub emergency: EmergencyStatus,
    pub save_dir: PathBuf,
    pub running_id: Option<ActionId>,
    pub actions: Vec<ActionInfo>,
}

#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum EmergencyStatus {
    #[default]
    None,
    WaitingForReset,
    Resetting,
}

pub type ActionId = u32;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionInfo {
    pub id: ActionId,
    pub type_name: String,
    pub save_dir: PathBuf,
    pub is_running: bool,
}
