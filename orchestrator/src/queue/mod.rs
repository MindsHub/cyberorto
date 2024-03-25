pub mod action_wrapper;

use std::{collections::VecDeque, sync::{Arc, Condvar, Mutex}};

use crate::{action::{emergency::EmergencyAction, Action}, state::StateHandler};

use self::action_wrapper::{ActionId, ActionWrapper};

#[derive(Debug, PartialEq)]
enum EmergencyStatus {
    None,
    WaitingForReset,
    Resetting,
}

#[derive(Debug)]
struct Queue {
    actions: VecDeque<ActionWrapper>,
    paused: bool,
    emergency: EmergencyStatus,
    id_counter: ActionId,
}

impl Queue {
    fn create_action_wrapper(&mut self, action: Box<dyn Action>) -> ActionWrapper {
        let res = ActionWrapper { action, id: self.id_counter };
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
    ($queue:expr, $queuevar:ident, $block:block) => {
        {
            let (queue, condvar) = &*$queue;
            let mut $queuevar = queue.lock().unwrap();
            let res = $block;
            condvar.notify_all();
            res
        }
    };
}

impl QueueHandler {
    pub fn new(state_handler: StateHandler) -> QueueHandler {
        QueueHandler {
            queue: Arc::new((Mutex::new(Queue {
                actions: VecDeque::new(),
                paused: false,
                emergency: EmergencyStatus::None,
                id_counter: 0,
            }), Condvar::new())),
            state_handler,
        }
    }

    fn get_current_action(&self, mut last_current_action: Option<ActionWrapper>) -> ActionWrapper {
        let (queue, condvar) = &*self.queue;
        let mut queue = queue.lock().unwrap();

        loop {
            if queue.paused || queue.emergency != EmergencyStatus::None {
                if let Some(current_action) = last_current_action {
                    queue.actions.push_front(current_action);
                    last_current_action = None;
                }
                if queue.emergency == EmergencyStatus::WaitingForReset {
                    queue.emergency = EmergencyStatus::Resetting;
                    return queue.create_action_wrapper(Box::new(EmergencyAction {}));
                }
            } else if let Some(current_action) = last_current_action {
                return current_action
            } else if let Some(current_action) = queue.actions.pop_front() {
                return current_action
            }

            queue = condvar.wait(queue).unwrap();
        }
    }

    pub fn main_loop(&self) {
        let mut last_current_action = None;
        loop {
            let mut current_action = self.get_current_action(last_current_action);
            if current_action.action.step(&self.state_handler) {
                last_current_action = Some(current_action);
            } else {
                last_current_action = None;
            }
        }
    }

    pub fn add_action(&self, action: Box<dyn Action>) -> ActionId {
        mutate_queue_and_notify!(self.queue, queue, {
            let action = queue.create_action_wrapper(action);
            let id = action.id;
            queue.actions.push_back(action);
            id
        })
    }
}
