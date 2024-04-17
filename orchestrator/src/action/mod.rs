pub mod emergency;
pub mod command_list;
pub mod action_wrapper;

use std::{any::{Any, TypeId}, fmt::Debug};

use crate::state::StateHandler;

use self::{action_wrapper::Context, emergency::EmergencyAction};

#[async_trait]
pub trait Action: Debug + Send {
    /// Returns `true` if there are some more steps available,
    /// or `false` if the action has finished executing.
    async fn step(&mut self, ctx: &Context, state_handler: &StateHandler) -> bool;

    // lifecycle
    fn acquire(&mut self, ctx: &Context) {}
    fn release(&mut self, ctx: &Context) {}

    // saving/loading to/from disk
    fn get_type_name() -> &'static str where Self: Sized;
    fn save_to_disk(&self, ctx: &Context) -> Result<(), String>;
    fn load_from_disk(ctx: &Context) -> Result<Self, String> where Self: Sized;
}
