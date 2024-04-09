pub mod emergency;
pub mod command_list;

use std::fmt::Debug;

use crate::state::StateHandler;

pub trait Action: Debug + Send {
    fn step(&mut self, state_handler: &StateHandler) -> bool;
}
