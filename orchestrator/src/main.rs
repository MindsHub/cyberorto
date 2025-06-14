use std::{env, path::PathBuf, thread, time::Duration};

use clap::Parser;
use embedcore::protocol::{cyber::Slave, test_harness::Testable};
use queue::QueueHandler;
use state::StateHandler;
use tokio::{signal::unix::{signal, SignalKind}, sync::oneshot};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use util::cors::Cors;

use crate::state::dummy_message_handler::DummyMessageHandler;

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
    #[arg(short, long, default_value_os_t = PathBuf::from(env::var("HOME").unwrap_or(".".to_string()) + "/.cyberorto/queue"))]
    queue_dir: PathBuf,
}

#[rocket::main]
async fn main() {
    let args = Args::parse();

    let (master, slave_handle) = if args.no_serial {
        let (master, slave) = SerialStream::pair()
            .expect("Failed to create dummy serial");

        let mut slave = Slave::new(slave, 1000, *b"test_slave", DummyMessageHandler::new());
        let slave_handle = tokio::task::spawn(async move { slave.run().await });

        (master, Some(slave_handle))
    } else {
        (tokio_serial::new(&args.port, 115200)
            .timeout(Duration::from_millis(3))
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()
            .expect("Failed to open port"), None)
    };

    let state_handler = StateHandler::new(master);
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

    if let Some(slave_handle) = slave_handle {
        slave_handle.abort();
    }
}
