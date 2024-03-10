use crate::state::State;
use std::thread;

use queue::QueueHandler;
use rocket::{request::{self, FromRequest}, Request};
use state::StateHandler;

mod action;
mod api;
mod queue;
mod state;
mod constants;

#[macro_use]
extern crate rocket;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for State {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request.guard::<&rocket::State<StateHandler>>().await
            .map(|request_handler| request_handler.get_state())
    }
}

// why doesn't Rocket provide directly &QueueHandler,
// but only &State<QueueHandler>, thus requiring this FromRequest impl?
#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r QueueHandler {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request.guard::<&rocket::State<QueueHandler>>().await
            .map(|request_handler| request_handler.inner())
    }
}

#[get("/")]
fn hello(robot_state: State, queue_handler: &QueueHandler) -> String {
    format!("Hello, {:?} {:?}!", robot_state, queue_handler)
}

#[rocket::main]
async fn main() {
    let state_handler = StateHandler::new();
    let queue_handler = QueueHandler::new(state_handler.clone());

    let queue_handler_clone = queue_handler.clone();
    let _queue_handler_thread = thread::spawn(move || {
        queue_handler_clone.main_loop()
    });

    rocket::build()
        .manage(state_handler)
        .manage(queue_handler)
        .mount("/hello", routes![hello])
        .launch()
        .await
        .unwrap();

    // launch().await will block until it receives a shutdown request (e.g. Ctrl+C)
    println!("Shutting down Cyberorto orchestrator...");
}
