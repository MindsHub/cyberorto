use std::{env, path::PathBuf, thread, time::Duration};

use clap::{builder::OsStr, command, Parser};
use queue::QueueHandler;
use state::StateHandler;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use util::cors::Cors;

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
    /// The port to connect to, where an Arduino should be listening
    #[arg(short, long, default_value = "/dev/ttyACM0")]
    port: String,

    /// Whether to skip connecting to the serial port, and instead just set up a dummy serial connection for testing
    #[arg(short, long, action)]
    no_serial: bool,

    /// The directory in which to save data about the queue
    #[arg(short, long, default_value_os_t = PathBuf::from(env::var("HOME").expect("$HOME must be set") + "/.cyberorto/queue"))]
    queue_dir: PathBuf,
}

#[rocket::main]
async fn main() {
    let args = Args::parse();

    let port = if args.no_serial {
        SerialStream::pair()
            .expect("Failed to create dummy serial")
            .0 // TODO start a dummy Arduino implementation on the other stream
    } else {
        tokio_serial::new(&args.port, 115200)
            .timeout(Duration::from_millis(3))
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()
            .expect("Failed to open port")
    };

    let state_handler = StateHandler::new(port);
    let queue_handler = QueueHandler::new(state_handler.clone(), args.queue_dir);

    let queue_handler_clone = queue_handler.clone();
    let queue_handler_thread = thread::spawn(move || queue_handler_clone.run());

    rocket::build()
        .attach(Cors)
        .manage(state_handler) // used by `impl FromRequest for State`
        .manage(queue_handler.clone()) // used by `impl FromRequest for &QueueHandler`
        .mount("/", routes![
            api::pause,
            api::unpause,
            api::clear,
            api::kill_running_action,
            api::get_state,
            api::toggle_led,
            api::add_action_command_list
        ])
        .launch()
        .await
        .unwrap();

    // launch().await will block until it receives a shutdown request (e.g. Ctrl+C)
    println!("Shutting down Cyberorto orchestrator...");
    queue_handler.stop();
    queue_handler_thread.join().unwrap();
}
