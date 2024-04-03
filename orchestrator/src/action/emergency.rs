use crate::state::StateHandler;

use super::Action;

#[derive(Debug)]
pub struct EmergencyAction {}

impl Action for EmergencyAction {
    fn step(&mut self, state_handler: &StateHandler) -> bool {
        // TODO implement better emergency logic
        state_handler.reset();
        false
    }
}
