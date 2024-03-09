use std::thread;

use queue::QueueHandler;
use state::StateHandler;

mod action;
mod api;
mod queue;
mod state;

fn main() {
    let state_handler = StateHandler::new();
    let queue_handler = QueueHandler::new(state_handler);

    let queue_handler_clone = queue_handler.clone();
    let queue_handler_thread = thread::spawn(move || {
        queue_handler_clone.main_loop()
    });

    // TODO instantiate API with ref to state_handler and queue

    queue_handler_thread.join().unwrap();
}
