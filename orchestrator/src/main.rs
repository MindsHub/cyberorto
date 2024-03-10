use std::thread;

use queue::QueueHandler;
use state::StateHandler;

mod action;
mod api;
mod queue;
mod state;
mod constants;

#[macro_use]
extern crate rocket;

#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[rocket::main]
async fn main() {
    let state_handler = StateHandler::new();
    let queue_handler = QueueHandler::new(state_handler);

    let queue_handler_clone = queue_handler.clone();
    let queue_handler_thread = thread::spawn(move || {
        queue_handler_clone.main_loop()
    });

    rocket::build()
        .mount("/hello", routes![hello])
        .launch()
        .await
        .unwrap();

    // launch().await will block until it receives a shutdown request (e.g. Ctrl+C)
    println!("Shutting down Cyberorto orchestrator...");
}
