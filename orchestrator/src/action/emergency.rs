use crate::state::StateHandler;

use super::{Action, Context};

/// Puts the robot in a safe anchored position, so that strong wind can't damage it.
#[derive(Debug)]
pub struct EmergencyAction {}

#[async_trait]
impl Action for EmergencyAction {
    async fn step(&mut self, ctx: &Context, state_handler: &StateHandler) -> bool {
        // TODO implement better emergency logic
        state_handler.reset();
        false
    }

    fn get_type_name() -> &'static str where Self: Sized {
        "emergency"
    }

    fn save_to_disk(&self, _ctx: &Context) -> Result<(), String> {
        Ok(())
    }

    fn load_from_disk(_ctx: &Context) -> Result<Self, String> where Self: Sized {
        Ok(EmergencyAction {})
    }
}
