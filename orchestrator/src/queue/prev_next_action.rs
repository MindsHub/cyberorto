use crate::action::{action_wrapper::Context, Action};

/// Just a helper struct with an optional action and its corresponding context, used to represent
/// the action that was just `step()`ped (which might be [None] if it has finished executing).
pub struct PrevAction {
    pub action: Option<Box<dyn Action>>,
    pub ctx: Context,
}

/// Just a helper struct with an action and its corresponding context, used to represent the next
/// action to execute.
pub struct NextAction {
    pub action: Box<dyn Action>,
    pub ctx: Context,
}
