use std::sync::{Arc, Mutex};

use crate::{action::Action, state::StateHandler};

#[derive(Debug)]
struct Queue {
    actions: Vec<Box<dyn Action>>,
    current_action: Option<Box<dyn Action>>,
    state_handler: StateHandler,
}

#[derive(Debug, Clone)]
pub struct QueueHandler {
    queue: Arc<Mutex<Queue>>,
    // TODO add serial object
}

impl QueueHandler {
    pub fn new(state_handler: StateHandler) -> QueueHandler {
        QueueHandler {
            queue: Arc::new(Mutex::new(Queue {
                actions: vec![],
                current_action: None,
                state_handler,
            })),
        }
    }

    pub fn main_loop(&self) {
        loop {
            let mut queue = self.queue.lock().unwrap();
            if let Some(current_action) = &mut queue.current_action {
                if !current_action.step() {
                    queue.current_action = None
                }
            }
            // TODO
        }
    }

    // TODO add functions to handle queue
}
