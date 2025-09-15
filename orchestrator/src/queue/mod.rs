mod tests;

use std::{
    collections::{HashMap, VecDeque}, fs::create_dir_all, future::Future, panic::{catch_unwind, AssertUnwindSafe, UnwindSafe}, path::PathBuf, sync::{Arc, Condvar, Mutex, MutexGuard}
};

use definitions::{ActionInfo, EmergencyStatus, QueueState, StepProgress};
use log::trace;
use rocket::futures::FutureExt;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::{
    action::{
        action_wrapper::{ActionId, ActionWrapper}, emergency::EmergencyAction, Action, StepResult
    },
    state::{StateHandler, StateHandlerError},
    util::serde::{deserialize_from_json_file, serialize_to_json_file},
};

#[derive(Debug)]
pub enum ReorderError {
    MismatchedExpectedNew,
    QueueChanged,
}

#[derive(Debug)]
struct Queue {
    actions: VecDeque<ActionWrapper>,
    paused: bool,
    stopped: bool,
    emergency: EmergencyStatus,
    running_id: Option<ActionId>,
    running_killer: Option<oneshot::Sender<bool>>,

    id_counter: ActionId,
    save_dir: PathBuf,
}

impl Queue {
    fn create_action_wrapper<A: Action + 'static>(&mut self, action: A) -> ActionWrapper {
        let res = ActionWrapper::new(action, self.id_counter, &self.save_dir);
        self.id_counter = self.id_counter.wrapping_add(1);
        res
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct QueueTestStats {
    wait_counter: usize,
    tick_counter: usize,
}

#[derive(Debug, Clone)]
pub struct QueueHandler {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    state_handler: StateHandler,
    #[cfg(test)]
    test_stats: Arc<Mutex<QueueTestStats>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QueueData {
    action_save_dirs: Vec<PathBuf>,
    id_counter: ActionId,
}

/// TODO: handle panics nicely with [std::panic::catch_unwind]
impl QueueHandler {
    pub fn new(state_handler: StateHandler, save_dir: PathBuf) -> QueueHandler {
        QueueHandler {
            queue: Arc::new((
                Mutex::new(Queue {
                    actions: VecDeque::new(),
                    paused: false,
                    stopped: false,
                    emergency: EmergencyStatus::None,
                    running_id: None,
                    running_killer: None,
                    id_counter: 0,
                    save_dir,
                }),
                Condvar::new(),
            )),
            state_handler,
            #[cfg(test)]
            test_stats: Arc::new(Mutex::new(QueueTestStats {
                wait_counter: 0,
                tick_counter: 0,
            })),
        }
    }

    #[cfg(test)]
    fn increase_wait_counter(&self) {
        self.test_stats.lock().unwrap().wait_counter += 1;
    }

    #[cfg(test)]
    fn increase_tick_counter(&self) {
        self.test_stats.lock().unwrap().tick_counter += 1;
    }

    /// Readds the last current action to the queue at the position where its
    /// placeholder is. This allows the current action to be moved around the
    /// queue or deleted even while it is being executed.
    ///
    /// This also calls [`release()`](Action::release) on the action, before
    /// putting it back into the queue, or before deleting it.
    ///
    /// Note that this function may temporarily release the lock in `queue`,
    /// since [`release()`](Action::release) may take some time to execute,
    /// and holding the lock for that time may cause problems. This is why in
    /// [`get_next_action()`](Self::get_next_action()) every call to this
    /// function should be followed by a `continue`, to allow all checks to
    /// be performed again.
    ///
    /// Returns the original lock (aka the parameter `queue`), or a new lock
    /// if the original lock was temporarily released while
    /// [`release()`](Action::release) was being executed.
    fn release_prev_action<'a>(
        &'a self,
        mut queue: MutexGuard<'a, Queue>,
        mut action: ActionWrapper,
    ) -> MutexGuard<'a, Queue> {
        if action.action.is_some() {
            // call release() on the action to save resources while it
            // is not being executed anymore (or if it has been deleted)
            drop(queue);
            action.action.as_mut().unwrap().release(&action.ctx);
            queue = self.queue.0.lock().unwrap();

            if let Some(item) = queue
                .actions
                .iter_mut()
                .find(|item| item.get_id() == action.get_id())
            {
                // If the placeholder corresponding to the current action is in the queue,
                // replace it with the non-placeholder current action. This not only moves
                // the Action object back in the queue, but also updates other fields in
                // ActionWrapper and effectively pauses the action if it's not going to be
                // taken again right after in the loop below.
                *item = action;
            } else {
                // `else`, it means that the placeholder has been deleted from the queue
                // in the meantime, so delete any data this action might have saved to disk
                // and just let it be dropped. The loop below will decide which action will
                // come next.
                action.delete_data_on_disk();
            }
        } else {
            // The current action has finished executing, delete any of its data on disk.
            action.delete_data_on_disk();

            if let Some(index) = queue
                .actions
                .iter()
                .position(|item| item.get_id() == action.get_id())
            {
                // If the current action has finished executing,
                // remove its corresponding placeholder from the queue.
                // No need to call release() since it has already been called
                // in the main loop.
                queue.actions.remove(index);
            }
            // `else`, the current action has finished executing and its corresponding
            // placeholder has already been deleted from the queue, so nothing to do
        }
        queue
    }

    fn get_next_action(&self, mut prev_action: Option<ActionWrapper>) -> Option<ActionWrapper> {
        let (queue_mutex, condvar) = &*self.queue;
        let mut queue = queue_mutex.lock().unwrap();

        loop {
            if queue.stopped {
                if let Some(prev_action) = std::mem::take(&mut prev_action) {
                    // the queue is being terminated,
                    // so release resources for the current action
                    queue = self.release_prev_action(queue, prev_action);
                    continue;
                }
                return None;
            } else if queue.paused || queue.emergency != EmergencyStatus::None {
                if queue.emergency == EmergencyStatus::WaitingForReset {
                    if let Some(prev_action) = std::mem::take(&mut prev_action) {
                        // we are pausing for a while during the emergency,
                        // so release resources for the current action
                        queue = self.release_prev_action(queue, prev_action);
                        continue;
                    }
                    queue.emergency = EmergencyStatus::Resetting;
                    return Some(queue.create_action_wrapper(EmergencyAction {}));
                }
            } else if let Some(id) = queue.actions.front().map(|a| a.get_id()) {
                if let Some(prev_action) = std::mem::take(&mut prev_action) {
                    if id == prev_action.get_id() && prev_action.action.is_some() {
                        // just continue executing the current action for another step
                        return Some(prev_action);
                    } else {
                        // the action to execute just changed, or the current action has finished
                        // executing, so release it
                        queue = self.release_prev_action(queue, prev_action);
                        continue;
                    }
                }

                // The id of the first action in the queue changed, so we are going to execute a new
                // action. The action is therefore extracted from the queue, and replaced with a
                // placeholder (i.e. an ActionWrapper with action=None)
                let action_in_queue = queue.actions.front_mut().unwrap();
                let mut next_action = ActionWrapper {
                    action: std::mem::take(&mut action_in_queue.action),
                    progress: action_in_queue.progress.clone(),
                    ctx: action_in_queue.ctx.clone(),
                };

                // Release the lock before calling `.acquire()`.
                drop(queue);

                // We call acquire() to abide by the action lifecycle.
                next_action
                    .action
                    .as_mut()
                    .expect("Unxpected placeholder in the queue")
                    .acquire(&next_action.ctx);
                return Some(next_action);
            }

            if let Some(prev_action) = std::mem::take(&mut prev_action) {
                // we are pausing for a while, so release resources for the current action
                queue = self.release_prev_action(queue, prev_action);
                continue;
            }

            #[cfg(test)]
            self.increase_wait_counter();
            queue = condvar.wait(queue).unwrap();
        }
    }

    /// Just a utility function to obtain a future from [`tokio::select`](tokio::select).
    /// Waits for `stepper` to finish executing, unless a message from `killer_rx` is received
    /// before `stepper` terminates.
    /// Returns:
    /// - [StepResult::Running] or [StepResult::RunningError] if there are some more steps
    ///   available,
    /// - [StepResult::Finished] or [StepResult::FinishedError] if the action has finished
    ///   executing, or if there was an unexpected panic.
    async fn step_or_kill<F: Future<Output = StepResult> + UnwindSafe>(
        stepper: F,
        killer_rx: oneshot::Receiver<bool>,
    ) -> StepResult {
        tokio::select! {
            output = stepper.catch_unwind() => {
                match output {
                    // Just forward the value from `stepper()`
                    Ok(output) => output,
                    // Some panic happened, do as if the action has finished executing
                    Err(err) => {
                        error!("Panic while stepping action: {err:?}");
                        StepResult::FinishedError(StateHandlerError::GenericError(
                            format!("Panic while stepping action: {err:?}")
                        ))
                    },
                }
            }

            // Calling unwrap() here, since the tx is never going to be dropped,
            // without sending anything, while this future is still being executed.
            // The tx is only dropped after it sends something, or after the other branch of
            // this select! has returned before this one.
            output = killer_rx => if output.unwrap() {
                info!("Received kill signal for action, with keep_in_queue = true");
                // keep in queue
                StepResult::RunningError(StateHandlerError::GenericError("Killed".to_string()))
            } else {
                info!("Received kill signal for action, with keep_in_queue = false");
                // remove from queue
                StepResult::FinishedError(StateHandlerError::GenericError("Killed".to_string()))
            },
        }
    }

    fn main_loop(&self) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let mut prev_action = None; // will be None only the first iteration
        loop {
            let action_wrapper = self.get_next_action(prev_action);
            #[cfg(test)]
            self.increase_tick_counter();

            if let Some(mut action_wrapper) = action_wrapper {
                // unwrapping since the returned action can't be a placeholder
                let id = action_wrapper.get_id();
                let action = action_wrapper.action.as_mut().unwrap();
                let (killer_tx, killer_rx) = oneshot::channel();

                {
                    let mut queue = self.queue.0.lock().unwrap();
                    queue.running_id = Some(id);
                    queue.running_killer = Some(killer_tx);
                }

                // `action.step()` returns `true` if there are some more steps available,
                // or `false` if the action has finished executing. The `killer_rx` channel
                // will also do the same, i.e. return `true` if the action should be kept
                // in the queue, or `false` otherwise.
                let step_result = runtime.block_on(Self::step_or_kill(
                    // Here we kindly ask the compiler to not check unwind safety. Losing unwind
                    // safety is not undefined behavior, though it could possibly lead to logic bugs
                    // if the panic happens in the middle of operations that leave `action` or
                    // `state_handler` in invalid states.
                    // See https://old.reddit.com/r/rust/comments/7diwt1 and
                    // https://users.rust-lang.org/t/73067/3 .
                    AssertUnwindSafe(action.step(&action_wrapper.ctx, &self.state_handler)),
                    killer_rx,
                ));

                let (should_release_action, should_pause_queue, new_progress) = match step_result {
                    StepResult::Running(step_progress) => {
                        // Nothing to do, action is kept in the queue.
                        trace!("action.step() with id {id} reported progress {step_progress:?}");
                        (false, false, step_progress)
                    },
                    StepResult::RunningError(state_handler_error) => {
                        // Action failed with an error, but should remain in queue, so pause the
                        // queue.
                        error!("action.step() with id {id} returned RunningError: {state_handler_error:?}");
                        (false, true, StepProgress::Unknown)
                    },
                    StepResult::Finished => {
                        trace!("action.step() with id {id} finished");
                        // TODO maybe allow returning progress here, too, instead of using 100% manually
                        (true, false, StepProgress::Percentage(1.0f32))
                    },
                    StepResult::FinishedError(state_handler_error) => {
                        error!("action.step() with id {id} returned FinishedError: {state_handler_error:?}");
                        (true, false, StepProgress::Unknown)
                    },
                };

                if should_release_action {
                    // The action has finished executing, release its resources and remove it from
                    // the queue. Run this with catch_unwind just in case there is some panic!() in
                    // action.release().
                    if let Err(err) = catch_unwind(AssertUnwindSafe(|| action.release(&action_wrapper.ctx))) {
                        error!("Panic while releasing action: {err:?}");
                        // ignore errors and proceed normally
                    }
                    action_wrapper.progress = StepProgress::Unknown;
                    action_wrapper.action = None;
                }

                {
                    let mut queue = self.queue.0.lock().unwrap();
                    queue.running_id = None;
                    queue.running_killer = None;
                    if should_pause_queue {
                        queue.paused = true;
                    }
                    if !matches!(new_progress, StepProgress::Unknown) {
                        if let Some(item) = queue
                            .actions
                            .iter_mut()
                            .find(|item| item.get_id() == id) {
                                item.progress = new_progress
                        }
                    }
                }
                prev_action = Some(action_wrapper);
            } else {
                return; // the queue was asked to stop
            }
        }
    }

    fn load_from_disk(&self) {
        let mut queue: MutexGuard<'_, Queue> = self.queue.0.lock().unwrap();
        let data = deserialize_from_json_file::<QueueData>(&queue.save_dir.join("queue.json"));
        let data = match data {
            Ok(data) => data,
            Err(e) => {
                // TODO log error
                println!("Error deserializing queue.json: {e}");
                return;
            }
        };

        queue.id_counter = data.id_counter;
        queue.actions.clear();
        for save_dir in data.action_save_dirs {
            match ActionWrapper::load_from_disk(&save_dir) {
                Ok(action) => queue.actions.push_back(action),
                Err(e) => {
                    // TODO log error
                    println!("Error deserializing action {save_dir:?}: {e}")
                }
            }
        }
    }

    fn save_to_disk(&self) {
        let queue = self.queue.0.lock().unwrap();
        let data = QueueData {
            action_save_dirs: queue
                .actions
                .iter()
                .map(|a| a.get_save_dir().clone())
                .collect(),
            id_counter: queue.id_counter,
        };

        if let Err(e) = create_dir_all(&queue.save_dir) {
            // TODO log error
            println!("Error creating save directory: {e}")
        }
        if let Err(e) = serialize_to_json_file(&data, &queue.save_dir.join("queue.json")) {
            // TODO log error
            println!("Error serializing queue.json: {e}")
        }

        for action in &queue.actions {
            // TODO log any error to disk
            if let Err(e) = action.save_to_disk() {
                println!("Error serializing action {:?}: {e}", action.ctx)
            }
        }
    }

    pub fn run(&self) {
        self.load_from_disk();
        self.main_loop();
        self.save_to_disk();
    }

    fn mutate_queue_and_notify<T>(&self, f: impl FnOnce(MutexGuard<'_, Queue>) -> T) -> T {
        let (queue, condvar) = &*self.queue;
        let res = f(queue.lock().unwrap());
        condvar.notify_all();
        res
    }

    pub fn add_action<A: Action + 'static>(&self, action: A) -> ActionId {
        self.mutate_queue_and_notify(|mut queue| {
            let action = queue.create_action_wrapper(action);
            let id = action.get_id();
            queue.actions.push_back(action);
            id
        })
    }

    pub fn reorder(&self, expected: Vec<ActionId>, new: Vec<ActionId>) -> Result<(), ReorderError> {
        {
            let mut expected_sorted = expected.clone();
            let mut new_sorted = new.clone();
            expected_sorted.sort();
            new_sorted.sort();
            if expected_sorted != new_sorted {
                return Err(ReorderError::MismatchedExpectedNew);
            }
        }

        self.mutate_queue_and_notify(|mut queue| {
            let current = queue
                .actions
                .iter()
                .map(|action| action.get_id())
                .collect::<Vec<ActionId>>();
            if expected != current {
                return Err(ReorderError::QueueChanged);
            }

            let count = new.len();
            let new: HashMap<ActionId, usize> = new
                .into_iter()
                .enumerate()
                .map(|(i, val)| (val, i))
                .collect();

            let q = &mut queue.actions;
            q.rotate_right(q.as_slices().1.len());
            q.as_mut_slices()
                .0
                .sort_by_key(|action| new.get(&action.get_id()).unwrap_or(&count));

            Ok(())
        })
    }

    pub fn clear(&self) {
        self.mutate_queue_and_notify(|mut queue| queue.actions.clear())
    }

    pub fn pause(&self) {
        self.mutate_queue_and_notify(|mut queue| queue.paused = true)
    }

    pub fn unpause(&self) {
        self.mutate_queue_and_notify(|mut queue| queue.paused = false)
    }

    pub fn stop(&self) {
        self.mutate_queue_and_notify(|mut queue| queue.stopped = true)
    }

    pub fn is_idle(&self) -> bool {
        let queue = self.queue.0.lock().unwrap();
        queue.running_id.is_none()
    }

    /// Always pauses the queue, and maintains it paused even after killing is finished.
    /// Then tries to kill the currently running action. Returns `true` if the action was
    /// killed successfully, or `false` otherwise.
    ///
    /// * `running_id` the id of the action that the caller thinks is currently
    ///   being executed; if this is not equal to the id of the action currently
    ///   being executed no action is killed, but the queue remains paused
    /// * `keep_in_queue` whether the killed action should be kept in queue after
    ///   being killed (which is possibly risky), or not
    pub fn kill_running_action(&self, running_id: ActionId, keep_in_queue: bool) -> bool {
        self.mutate_queue_and_notify(|mut queue| {
            queue.paused = true;
            if queue.running_id == Some(running_id) {
                if let Some(running_killer) = queue.running_killer.take() {
                    return running_killer.send(keep_in_queue).is_ok();
                }
            }
            false
        })
    }

    pub fn force_kill_any_running_action(&self) {
        self.mutate_queue_and_notify(|mut queue| {
            queue.paused = true;
            if let Some(running_killer) = queue.running_killer.take() {
                let _ = running_killer.send(false);
            }
        });
    }

    pub fn get_state(&self) -> QueueState {
        let queue = self.queue.0.lock().unwrap();
        QueueState {
            paused: queue.paused,
            stopped: queue.stopped,
            emergency: queue.emergency,
            save_dir: queue.save_dir.clone(),
            running_id: queue.running_id,
            actions: queue.actions.iter().map(|action| ActionInfo {
                id: action.get_id(),
                type_name: action.get_type_name().clone(),
                save_dir: action.get_save_dir().clone(),
                is_running: action.is_placeholder(),
                progress: action.get_progress().clone(),
            }).collect(),
        }
    }
}
