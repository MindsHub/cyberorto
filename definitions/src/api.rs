use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

use crate::{QueueState, RobotState};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RobotQueueState {
    #[serde(flatten)]
    pub robot: RobotState,
    pub queue: QueueState,
}

impl DerefMut for RobotQueueState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.robot
    }
}

impl Deref for RobotQueueState {
    type Target = RobotState;
    fn deref(&self) -> &Self::Target {
        &self.robot
    }
}
