use crate::{action::StepResult, state::StateHandler};

use super::{Action, Context};

/// Puts the robot in a safe anchored position, so that strong wind can't damage it.
#[derive(Debug)]
pub struct EmergencyAction {}

#[async_trait]
impl Action for EmergencyAction {
    async fn step(&mut self, _ctx: &Context, state_handler: &StateHandler) -> StepResult {
        // TODO implement better emergency logic
        if let Err(e) = state_handler.reset().await {
            error!("EmergencyAction could not reset: {e:?}")
        }
        StepResult::Finished
    }

    fn get_type_name() -> &'static str
    where
        Self: Sized,
    {
        "emergency"
    }

    fn save_to_disk(&self, _ctx: &Context) -> Result<(), String> {
        Ok(())
    }

    fn load_from_disk(_ctx: &Context) -> Result<Self, String>
    where
        Self: Sized,
    {
        Ok(EmergencyAction {})
    }
}
