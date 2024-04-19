#![allow(unused)] // TODO remove

use std::{env, path::PathBuf, thread, time::Duration};

use clap::{command, Parser};
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    port: Option<String>,

    #[arg(short, long)]
    save_dir: Option<PathBuf>,
}

#[rocket::main]
async fn main() {
    let args = Args::parse();
    let port = args.port.unwrap_or("/dev/ttyACM0".to_string());
    let save_dir = args.save_dir.unwrap_or_else(|| {
        let home = env::var("HOME").expect("$HOME must be set");
        PathBuf::from(home + "/.cyberorto/queue")
    });

    let port = serialport::new(&port, 115200)
        .timeout(Duration::from_millis(3))
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .open_native()
        .expect("Failed to open port");
    let state_handler = StateHandler::new(port);
    let queue_handler = QueueHandler::new(state_handler.clone(), save_dir);

    let queue_handler_clone = queue_handler.clone();
    let queue_handler_thread = thread::spawn(move || queue_handler_clone.run());

    rocket::build()
        .manage(state_handler) // used by `impl FromRequest for State`
        .manage(queue_handler.clone()) // used by `impl FromRequest for &QueueHandler`
        .mount("/", routes![api::toggle_led])
        .launch()
        .await
        .unwrap();

    // launch().await will block until it receives a shutdown request (e.g. Ctrl+C)
    println!("Shutting down Cyberorto orchestrator...");
    queue_handler.stop();
    queue_handler_thread.join().unwrap();
}
