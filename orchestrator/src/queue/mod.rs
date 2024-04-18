mod tests;

use std::{
    collections::{HashMap, VecDeque}, f32::consts::E, fs::{self, create_dir_all}, future::Future, path::PathBuf, sync::{Arc, Condvar, Mutex, MutexGuard}
};

use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use crate::{
    action::{action_wrapper::{self, ActionId, ActionWrapper}, emergency::EmergencyAction, Action},
    state::StateHandler, util::serde::{deserialize_from_json_file, serialize_to_json_file},
};

#[derive(Debug, PartialEq)]
enum EmergencyStatus {
    None,
    WaitingForReset,
    Resetting,
}

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

#[derive(Debug, Clone)]
pub struct QueueHandler {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    state_handler: StateHandler,
    // TODO add serial object
}

macro_rules! mutate_queue_and_notify {
    ($queue:expr, $queuevar:ident, $block:block) => {{
        let (queue, condvar) = &*$queue;
        let mut $queuevar = queue.lock().unwrap();
        let res = $block;
        condvar.notify_all();
        res
    }};
}

#[derive(Debug, Serialize, Deserialize)]
struct QueueData {
    action_save_dirs: Vec<PathBuf>,
    id_counter: ActionId,
}

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
        }
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
    fn release_prev_action<'a>(&'a self, mut queue: MutexGuard<'a, Queue>, mut action: ActionWrapper) -> MutexGuard<'a, Queue> {
        if action.action.is_some() {
            // call release() on the action to save resources while it
            // is not being executed anymore (or if it has been deleted)
            drop(queue);
            action.action.as_mut().unwrap().release(&action.ctx);
            queue = self.queue.0.lock().unwrap();

            if let Some(item) = queue.actions.iter_mut().find(|item| item.get_id() == action.get_id()) {
                // If the placeholder corresponding to the current action is in the queue,
                // replace it with the non-placeholder current action. This not only moves
                // the Action object back in the queue, but also updates other fields in
                // ActionWrapper and effectively pauses the action if it's not going to be
                // taken again right after in the loop below.
                *item = action;
            }
            // `else`, it means that the placeholder has been deleted from the queue
            // in the meantime, so just let the current action be dropped. The loop below
            // will decide which action will come next.
        } else if let Some(index) = queue.actions.iter().position(|item| item.get_id() == action.get_id()) {
            // If the current action has finished executing,
            // remove its corresponding placeholder from the queue.
            // No need to call release() since it has already been called
            // in the main loop.
            queue.actions.remove(index);
        }
        // `else`, the current action has finished executing and its corresponding
        // placeholder has already been deleted from the queue, so nothing to do
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
                let mut action_in_queue = queue.actions.front_mut().unwrap();
                let mut next_action = ActionWrapper {
                    action: std::mem::take(&mut action_in_queue.action),
                    ctx: action_in_queue.ctx.clone(),
                };

                // Release the lock before calling `.acquire()`.
                drop(queue);

                // We call acquire() to abide by the action lifecycle.
                next_action.action.as_mut()
                    .expect("Unxpected placeholder in the queue")
                    .acquire(&next_action.ctx);
                return Some(next_action);
            }

            if let Some(prev_action) = std::mem::take(&mut prev_action) {
                // we are pausing for a while, so release resources for the current action
                queue = self.release_prev_action(queue, prev_action);
                continue;
            }
            queue = condvar.wait(queue).unwrap();
        }
    }

    /// Just a utility function to obtain a future from [`tokio::select`](tokio::select).
    /// Waits for `stepper` to finish executing, unless a message from `killer_rx` is received
    /// before `stepper` terminates.
    /// Returns `true` if there are some more steps available,
    /// or `false` if the action has finished executing.
    async fn step_or_kill<F: Future<Output = bool>>(stepper: F, killer_rx: oneshot::Receiver<bool>) -> bool {
        tokio::select! {
            // Just forward the value from `stepper()`
            output = stepper => output,

            // Calling unwrap() here, since the tx is never going to be dropped,
            // without sending anything, while this future is still being executed.
            // The tx is only dropped after it sends something, or after the other branch of
            // this select! has returned before this one.
            output = killer_rx => output.unwrap(),
        }
    }

    fn main_loop(&self) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let mut prev_action = None; // will be None only the first iteration
        loop {
            let mut action_wrapper = self.get_next_action(prev_action);

            if let Some(mut action_wrapper) = action_wrapper {
                // unwrapping since the returned action can't be a placeholder
                let id = action_wrapper.get_id();
                let mut action = action_wrapper.action.as_mut().unwrap();
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
                if !runtime.block_on(
                    Self::step_or_kill(
                        action.step(&action_wrapper.ctx, &self.state_handler),
                        killer_rx,
                    )
                ) {
                    // The action has finished executing, release its resources and remove
                    // it from the queue.
                    action.release(&action_wrapper.ctx);
                    action_wrapper.action = None;
                }

                {
                    let mut queue = self.queue.0.lock().unwrap();
                    queue.running_id = None;
                    queue.running_killer = None;
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
            },
        };

        queue.id_counter = data.id_counter;
        queue.actions.clear();
        for save_dir in data.action_save_dirs {
            match ActionWrapper::load_from_disk(&save_dir) {
                Ok(action) => queue.actions.push_back(action),
                Err(e) => {
                    // TODO log error
                    println!("Error deserializing action {:?}: {}", save_dir, e)
                },
            }
        }
    }

    fn save_to_disk(&self) {
        let queue = self.queue.0.lock().unwrap();
        let data = QueueData {
            action_save_dirs: queue.actions.iter().map(|a| a.get_save_dir().clone()).collect(),
            id_counter: queue.id_counter,
        };

        create_dir_all(&queue.save_dir);
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

    pub fn add_action<A: Action + 'static>(&self, action: A) -> ActionId {
        mutate_queue_and_notify!(self.queue, queue, {
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

        mutate_queue_and_notify!(self.queue, queue, {
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
                .sort_by_key(|action| new.get(&action.get_id()).unwrap_or(&count))
        });

        Ok(())
    }

    pub fn clear(&self) {
        mutate_queue_and_notify!(self.queue, queue, {
            queue.actions.clear()
        })
    }

    pub fn pause(&self) {
        mutate_queue_and_notify!(self.queue, queue, {
            queue.paused = true
        })
    }

    pub fn unpause(&self) {
        mutate_queue_and_notify!(self.queue, queue, {
            queue.paused = false
        })
    }

    pub fn stop(&self) {
        mutate_queue_and_notify!(self.queue, queue, {
            queue.stopped = true
        })
    }

    /// Always pauses the queue. Then tries to kill the currently running action.
    /// Returns `true` if the action was killed successfully, or `false` otherwise.
    ///
    /// * `running_id` the id of the action that the caller thinks is currently
    ///                being executed; if this is not equal to the id of the
    ///                action currently being executed
    /// * `keep_in_queue` whether the killed action should be kept in queue after
    ///                   being killed (which is possibly risky), or not
    pub fn kill_running_action(&self, running_id: ActionId, keep_in_queue: bool) -> bool {
        mutate_queue_and_notify!(self.queue, queue, {
            queue.paused = true;
            if queue.running_id == Some(running_id) {
                if let Some(running_killer) = queue.running_killer.take() {
                    return running_killer.send(keep_in_queue).is_ok();
                }
            }
            return false;
        })
    }
}
