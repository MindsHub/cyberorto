pub mod action_wrapper;
pub mod command_list;
pub mod emergency;

use std::fmt::Debug;

use crate::state::StateHandler;

use self::action_wrapper::Context;

/// An "action" that the robot can perform, possibly made up of multiple
/// steps to allow pausing in-between. While being paused, the state of
/// an `Action` can be saved/loaded to/from disk.
#[async_trait]
pub trait Action: Debug + Send {
    //----
    // LIFECYCLE
    //----

    /// Returns `true` if there are some more steps available,
    /// or `false` if the action has finished executing.
    ///
    /// Will be called only right after [`acquire()`](Action::acquire()),
    /// or after other [`step()`](Action::step()) calls.
    ///
    /// * `ctx` contains information on this action, e.g. its id, the save folder, ...
    /// * `state_handler` allows performing operations that affect the state,
    ///                   e.g. moving the robot, opening water, ...
    async fn step(&mut self, ctx: &Context, state_handler: &StateHandler) -> bool;

    /// Acquires any resource that this Action needs in order to run.
    ///
    /// Will be called only before the first call to [`step()`](Action::step()),
    /// or right after a call to [`release()`](Action::release()). Will never be
    /// called multiple times in a row, but may be called again after a
    /// [`release()`](Action::release()).
    ///
    /// * `ctx` contains information on this action, e.g. its id, the save folder, ...
    fn acquire(&mut self, _ctx: &Context) {}

    /// Releases any resource that this Action needs in order to run,
    /// to save RAM and allow other actions/programs to use the same
    /// resources.
    ///
    /// Will be called when this action is temporarily paused, or when it is about
    /// to be dismissed. Therefore, will be called only after
    /// [`acquire()`](Action::acquire()) was called, and will never be called
    /// multiple times in a row.
    ///
    /// * `ctx` contains information on this action, e.g. its id, the save folder, ...
    fn release(&mut self, _ctx: &Context) {}

    //----
    // SAVING/LOADING TO/FROM DISK
    //----

    /// Returns the name associated with the type of this action. Must be a constant
    /// string, that does not change during the execution or even among different
    /// executions of the program, so that it can be used to save/load to/from disk.
    ///
    /// Will be used to keep track of the action type before calling
    /// [`save_to_disk()`](Action::save_to_disk()), and will be used call
    /// [`load_from_disk()`](Action::load_from_disk()) on the correct action type.
    fn get_type_name() -> &'static str
    where
        Self: Sized;

    /// Saves an action to disk, in order to persist actions among executions
    /// of the program.
    ///
    /// Stores data only in the [`ctx.save_dir`](Context::save_dir) directory.
    ///
    /// * `ctx` contains information on this action, e.g. its id, the save folder, ...
    fn save_to_disk(&self, ctx: &Context) -> Result<(), String>;

    /// Restores an action by reading data from disk.
    ///
    /// Reads data from the [`ctx.save_dir`](Context::save_dir) directory.
    ///
    /// * `ctx` contains information on this action, e.g. its id, the save folder, ...
    fn load_from_disk(ctx: &Context) -> Result<Self, String>
    where
        Self: Sized;
}
