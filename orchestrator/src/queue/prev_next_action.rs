use crate::action::{action_wrapper::Context, Action};

/// Just a helper struct with an optional action and its corresponding context, used to represent
/// the action that was just `step()`ped (which might be [None] if it has finished executing).
/// Basically a stripped down version of [crate::action::action_wrapper::ActionWrapper].
pub struct PrevAction {
    pub action: Option<Box<dyn Action>>,
    pub ctx: Context,
}

/// Just a helper struct with an action and its corresponding context, used to represent the next
/// action to execute. Basically a stripped down version of
/// [crate::action::action_wrapper::ActionWrapper], with a non-optional action.
pub struct NextAction {
    pub action: Box<dyn Action>,
    pub ctx: Context,
}
