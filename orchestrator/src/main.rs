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


#[rocket::main]
async fn main() {
    let state_handler = StateHandler::new();
    let queue_handler = QueueHandler::new(state_handler.clone());

    let queue_handler_clone = queue_handler.clone();
    let _queue_handler_thread = thread::spawn(move || {
        queue_handler_clone.main_loop()
    });

    rocket::build()
        .manage(state_handler) // used by `impl FromRequest for State`
        .manage(queue_handler) // used by `impl FromRequest for &QueueHandler`
        .mount("/hello", routes![api::hello])
        .launch()
        .await
        .unwrap();

    // launch().await will block until it receives a shutdown request (e.g. Ctrl+C)
    println!("Shutting down Cyberorto orchestrator...");
}
