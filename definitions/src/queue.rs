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
    pub progress: StepProgress,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum StepProgress {
    /// The progress is completely unknown.
    #[default]
    Unknown,
    /// Only the number of steps done so far is known, but the total number is unknown.
    Count {
        steps_done_so_far: usize,
    },
    /// The progress as a ratio between the number of steps already
    /// performed over the total number of steps.
    Ratio {
        steps_done_so_far: usize,
        steps_total: usize,
    },
    /// The progress as a number between 0 and 1.
    Percentage(f32),
}
