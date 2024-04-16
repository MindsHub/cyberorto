use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Condvar, Mutex},
};

use crate::{
    action::{action_wrapper::{self, ActionId, ActionWrapper}, emergency::EmergencyAction, Action},
    state::StateHandler,
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
    emergency: EmergencyStatus,
    id_counter: ActionId,
}

impl Queue {
    fn create_action_wrapper<A: Action + 'static>(&mut self, action: A) -> ActionWrapper {
        let res = ActionWrapper::new(action, self.id_counter);
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

impl QueueHandler {
    pub fn new(state_handler: StateHandler) -> QueueHandler {
        QueueHandler {
            queue: Arc::new((
                Mutex::new(Queue {
                    actions: VecDeque::new(),
                    paused: false,
                    emergency: EmergencyStatus::None,
                    id_counter: 0,
                }),
                Condvar::new(),
            )),
            state_handler,
        }
    }

    /// Readds the last current action to the queue at the position where its
    /// placeholder is. This allows the current action to be moved around the
    /// queue even while it is being executed. This also calls
    /// [release\(\)](Action::release) on the action before putting it back
    /// into the queue.
    fn release_last_current_action(queue: &mut Queue, mut action: ActionWrapper) {
        if action.action.is_some() {
            // call release() on the action to save resources while it
            // is not being executed anymore (or if it has been deleted)
            action.action.as_mut().unwrap().release(&action.ctx);

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
    }

    fn get_current_action(&self, mut last_current_action: Option<ActionWrapper>) -> ActionWrapper {
        let (queue, condvar) = &*self.queue;
        let mut queue = queue.lock().unwrap();

        loop {
            if queue.paused || queue.emergency != EmergencyStatus::None {
                if queue.emergency == EmergencyStatus::WaitingForReset {
                    queue.emergency = EmergencyStatus::Resetting;
                    if let Some(last_current_action) = std::mem::take(&mut last_current_action) {
                        // we are pausing for a while during the emergency,
                        // so release resources for the current action
                        Self::release_last_current_action(&mut queue, last_current_action)
                    }
                    return queue.create_action_wrapper(EmergencyAction {});
                }
            } else if let Some(id) = queue.actions.front().map(|a| a.get_id()) {
                if let Some(last_current_action) = std::mem::take(&mut last_current_action) {
                    if id == last_current_action.get_id() && last_current_action.action.is_some() {
                        // just continue executing the current action for another step
                        return last_current_action;
                    } else {
                        // the action to execute just changed, or the current action has finished
                        // executing, so release it
                        Self::release_last_current_action(&mut queue, last_current_action)
                    }
                }

                // The id of the first action in the queue changed, so we are going to execute a new
                // action. The action is therefore extracted from the queue, and replaced with a
                // placeholder (i.e. an ActionWrapper with action=None)
                let mut new_current_action = queue.actions.front_mut().unwrap();
                let mut new_current_action = ActionWrapper {
                    action: std::mem::take(&mut new_current_action.action),
                    ctx: new_current_action.ctx.clone(),
                };

                // We call acquire() to abide by the action lifecycle.
                new_current_action.action.as_mut()
                    .expect("Unxpected placeholder in the queue")
                    .acquire(&new_current_action.ctx);
                return new_current_action;
            }

            if let Some(last_current_action) = std::mem::take(&mut last_current_action) {
                // we are pausing for a while, so release resources for the current action
                Self::release_last_current_action(&mut queue, last_current_action)
            }
            queue = condvar.wait(queue).unwrap();
        }
    }

    pub fn main_loop(&self) {
        let mut last_action = None; // will be None only the first iteration
        loop {
            let mut action_wrapper = self.get_current_action(last_action);
            // unwrapping since the returned action can't be a placeholder
            let mut action = action_wrapper.action.as_mut().unwrap();
            if !action.step(&action_wrapper.ctx, &self.state_handler) {
                action.release(&action_wrapper.ctx);
                action_wrapper.action = None;
            }
            last_action = Some(action_wrapper);
        }
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
}
