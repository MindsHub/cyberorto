use crate::State;
use crate::queue::QueueHandler;

mod from_request;

#[get("/")]
pub fn hello(robot_state: State, queue_handler: &QueueHandler) -> String {
    format!("Hello, {:?} {:?}!", robot_state, queue_handler)
}
