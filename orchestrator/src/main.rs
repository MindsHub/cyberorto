use std::{env, path::PathBuf, thread};

use clap::Parser;
use env_logger::Env;
use queue::QueueHandler;
use state::StateHandler;
use tokio::{signal::unix::{signal, SignalKind}, sync::oneshot};
use util::cors::Cors;

use crate::util::serial::SerialPorts;

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
    /// The ports to connect to, where a device implementing the cyberorto messaging
    /// protocol should be listening. Accepts one of these:
    /// - "auto" to autodiscover ports (default),
    /// - "simulated" to simulate connecting to fake motors and fake peripherals,
    /// - "PORT1,PORT2" to specify comma separated port names (e.g. "/dev/ttyACM0")
    ///
    /// The type (i.e. motor x, y, z or peripherals) of each connected device will be determined
    /// based on their name automatically.
    ///
    /// The port baud rate will always be 115200.
    #[arg(short, long, value_parser = SerialPorts::parse, default_value = "auto")]
    ports: SerialPorts,

    /// The directory in which to save data about the queue
    #[arg(short, long, default_value_os_t = PathBuf::from(env::var("HOME").unwrap_or(".".to_string()) + "/.cyberorto/queue"))]
    queue_dir: PathBuf,
}

#[rocket::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    log::info!("Cyberorto orchestrator starting...");
    let args = Args::parse();

    // TODO use 4 different serial ports: x, y, z, sensors

    let (masters, simulation_join_handles) = args.ports.to_masters();

    let state_handler = StateHandler::new(masters);
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
    info!("Shutting down Cyberorto orchestrator...");
    if !queue_handler.is_idle() {
        warn!("An action is running, waiting for it to give back control gracefully...");
        warn!("Press Ctrl+C again to force kill and delete any running action");
    }

    // this just tells the current action to stop after it has finished its current step,
    // but the current action will still remain in the queue and will resume next time
    // the orchestrator is started
    queue_handler.stop();

    // this spawns an async task that waits for another Ctrl+C
    let (sigint_stop_tx, sigint_stop_rx) = oneshot::channel();
    let sigint_thread = tokio::task::spawn(
        async move {
            let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();
            let received = tokio::select! {
                it = signal_interrupt.recv() => it,
                _ = sigint_stop_rx => None,
            };
            if received.is_some() {
                warn!("Force killing and deleting the currently running action...");
                queue_handler.force_kill_any_running_action()
            }
        }
    );

    // this blocks until the currently running action has finished its current step,
    // but if that step never finishes (e.g. waiting for 1000000s), then pressing
    // Ctrl+C again will trigger the code above which force kills any running action.
    queue_handler_thread.join().unwrap();

    // join the final thread listening for a Ctrl+C
    let _ = sigint_stop_tx.send(());
    sigint_thread.await.unwrap();

    for join_handle in simulation_join_handles {
        join_handle.abort();
    }
}
