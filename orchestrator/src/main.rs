use std::{env, path::PathBuf, thread};

use clap::Parser;
use env_logger::Env;
use queue::QueueHandler;
use state::StateHandler;
use tokio::{signal::unix::{signal, SignalKind}, sync::oneshot};
use util::cors::Cors;

use crate::{state::parameters::{load_parameters_from_disk, save_parameters_to_disk}, util::{serial::SerialPorts, test_devices::test_devices}};

mod action;
mod api;
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

    /// The directory in which to save cyberorto data (e.g. the state of the queue).
    #[arg(short, long, default_value_os_t = PathBuf::from(env::var("HOME").unwrap_or(".".to_string()) + "/.cyberorto"))]
    data_dir: PathBuf,

    /// Whether to save parameters to `${data_dir}/parameters.json` when the orchestrator is
    /// shutting down. Note: the JSON file will be rewritten from scratch, so any comments will be
    /// lost. That's why this field is `false` by default.
    #[arg(short, long)]
    save_parameters: bool,

    /// If this option is passed, the orchestrator will not start, and instead some checks will be
    /// performed on connected serial port peripherals, to check if they work and print some
    /// information about them. Some of the other args are useless if this option is passed.
    #[arg(short, long)]
    test_devices: bool,
}

#[rocket::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    if args.test_devices {
        test_devices(args.ports).await;
        return;
    }

    log::info!("Cyberorto orchestrator starting...");


    let (masters, simulation_join_handles) = args.ports.to_masters().await;
    let parameters = load_parameters_from_disk(&args.data_dir);
    let state_handler = StateHandler::new(masters, parameters);
    let queue_handler = QueueHandler::new(state_handler.clone(), args.data_dir.join("queue/"));

    let queue_handler_clone = queue_handler.clone();
    let queue_handler_thread = thread::spawn(move || queue_handler_clone.run());

    // start Rocket
    let rocket_error = rocket::build()
        .attach(Cors)
        .manage(state_handler.clone()) // used by `impl FromRequest for State`
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
        .await;
    if let Err(e) = rocket_error {
        error!("Could not start Rocket: {e:?}");
    }

    // save parameters (note that this will reformat the file and delete comments!)
    if args.save_parameters {
        save_parameters_to_disk(&state_handler.get_state().parameters, &args.data_dir);
    }

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
