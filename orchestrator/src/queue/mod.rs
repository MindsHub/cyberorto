use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Condvar, Mutex},
};

use crate::{
    action::{action_wrapper::{ActionId, ActionWrapper}, emergency::EmergencyAction, Action},
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

    fn get_current_action(&self, last_current_action: Option<ActionWrapper>) -> ActionWrapper {
        let (queue, condvar) = &*self.queue;
        let mut queue = queue.lock().unwrap();

        // Readd the last current action to the queue at the position where its placeholder is.
        // This allows moving even the action being currently executed.
        if let Some(action) = last_current_action {
            if action.action.is_some() {
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
                queue.actions.remove(index);
            }
            // `else`, the current action has finished executing and its corresponding
            // placeholder has already been deleted from the queue, so nothing to do
        }

        loop {
            if queue.paused || queue.emergency != EmergencyStatus::None {
                if queue.emergency == EmergencyStatus::WaitingForReset {
                    queue.emergency = EmergencyStatus::Resetting;
                    return queue.create_action_wrapper(EmergencyAction {});
                }
            } else if let Some(current_action) = queue.actions.front_mut() {
                return current_action.make_placeholder_and_extract();
            }

            queue = condvar.wait(queue).unwrap();
        }
    }

    pub fn main_loop(&self) {
        let mut last_action = None; // will be None only the first iteration
        loop {
            let mut action = self.get_current_action(last_action);
            // unwrapping since the returned action can't be a placeholder
            if !action.action.as_mut().unwrap().step(&action.ctx, &self.state_handler) {
                action.action = None;
            }
            last_action = Some(action);
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
