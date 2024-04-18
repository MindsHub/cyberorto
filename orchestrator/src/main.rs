#![allow(unused)] // TODO remove

use std::{env, path::PathBuf, thread, time::Duration};

use queue::QueueHandler;
use state::StateHandler;

mod action;
mod api;
mod constants;
mod queue;
mod state;
mod util;

#[macro_use]
extern crate rocket;

#[rocket::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();
    let tty = args.get(1).unwrap_or(&"/dev/ttyACM0".to_string()).clone();

    let port = serialport::new(&tty, 115200)
        .timeout(Duration::from_millis(3))
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .open_native()
        .expect("Failed to open port");
    let state_handler = StateHandler::new(port);
    let queue_handler = QueueHandler::new(state_handler.clone(), PathBuf::from("~/.cyberorto/queue"));

    let queue_handler_clone = queue_handler.clone();
    let _queue_handler_thread = thread::spawn(move || queue_handler_clone.run());

    rocket::build()
        .manage(state_handler) // used by `impl FromRequest for State`
        .manage(queue_handler) // used by `impl FromRequest for &QueueHandler`
        .mount("/", routes![api::toggle_led])
        .launch()
        .await
        .unwrap();

    // launch().await will block until it receives a shutdown request (e.g. Ctrl+C)
    println!("Shutting down Cyberorto orchestrator...");
}
