use std::{collections::VecDeque, sync::{Arc, Condvar, Mutex}};

use crate::{action::{emergency::EmergencyAction, Action}, state::StateHandler};

#[derive(Debug, PartialEq)]
enum EmergencyStatus {
    None,
    WaitingForReset,
    Resetting,
}

#[derive(Debug)]
struct Queue {
    actions: VecDeque<Box<dyn Action>>,
    paused: bool,
    emergency: EmergencyStatus,
}

#[derive(Debug, Clone)]
pub struct QueueHandler {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    state_handler: StateHandler,
    // TODO add serial object
}

impl QueueHandler {
    pub fn new(state_handler: StateHandler) -> QueueHandler {
        QueueHandler {
            queue: Arc::new((Mutex::new(Queue {
                actions: VecDeque::new(),
                paused: false,
                emergency: EmergencyStatus::None,
            }), Condvar::new())),
            state_handler,
        }
    }

    fn get_current_action(&self, mut last_current_action: Option<Box<dyn Action>>) -> Box<dyn Action> {
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
                    return Box::new(EmergencyAction {});
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
            if current_action.step(&self.state_handler) {
                last_current_action = Some(current_action);
            } else {
                last_current_action = None;
            }
        }
    }

    // TODO add functions to handle queue
}
