use std::{process::exit, sync::Arc, time::Duration};

use embedcore::{common::controllers::pid::CalibrationMode, protocol::cyber::{Master, Slave}};
use rocket::futures::never::Never;
use tokio::task::JoinHandle;
use tokio_serial::SerialStream;

use crate::state::dummy_message_handler::DummyMessageHandler;



#[derive(Debug, Clone)]
pub enum SerialPorts {
    Simulated,
    Autodiscover,
    Ports(Vec<String>),
}

pub struct Masters {
    pub x: Arc<Master<SerialStream>>,
    pub y: Arc<Master<SerialStream>>,
    pub z: Arc<Master<SerialStream>>,
    pub peripherals: Arc<Master<SerialStream>>,
}

impl SerialPorts {
    pub fn parse(s: &str) -> Result<SerialPorts, String> {
        if s == "simulated" {
            Ok(SerialPorts::Simulated)
        } else if s == "auto" {
            Ok(SerialPorts::Autodiscover)
        } else {
            Ok(SerialPorts::Ports(s.split(',').map(str::to_string).collect()))
        }
    }

    pub fn to_masters(&self) -> (Masters, Vec<JoinHandle<Never>>) {
        match self {
            SerialPorts::Simulated => Self::to_masters_simulated(),
            SerialPorts::Autodiscover => (Self::to_masters_autodiscover(), vec![]),
            SerialPorts::Ports(ports) => (Self::to_masters_ports(ports), vec![]),
        }
    }

    fn to_masters_simulated() -> (Masters, Vec<JoinHandle<Never>>) {
        let mut masters = vec![];
        let mut motors = vec![];
        let mut join_handles = vec![];
        for name in [b"x         ", b"y         ", b"z         ", b"p         "] {
            let (master, slave) = SerialStream::pair()
                .expect("Failed to create dummy serial");
            masters.push(Arc::new(Master::new(master, 100000, 20)));

            let (dummy_message_handler, motor) = DummyMessageHandler::new();
            let mut slave = Slave::new(slave, 1000, *name, dummy_message_handler);
            join_handles.push(tokio::task::spawn(async move { slave.run().await }));
            if name != b"p         " {
                // the last
                motors.push(motor);
            }
        }
        assert_eq!(4, masters.len());
        assert_eq!(3, motors.len());

        // update all motors in a single task for simplicity
        join_handles.push(tokio::task::spawn(async move {
            for motor in &motors {
                motor.lock().await.calibration(0, CalibrationMode::NoOvershoot).await;
            }
            let mut ticker = tokio::time::interval(Duration::from_millis(1));
            loop {
                for motor in &motors {
                    motor.lock().await.update().await;
                    //println!("Updating motor, pos = {:?}", motor.lock().await.pid);
                }
                ticker.tick().await;
            }
        }));

        let x = masters.remove(0);
        let y = masters.remove(0);
        let z = masters.remove(0);
        let peripherals = masters.remove(0);
        (Masters { x, y, z, peripherals }, join_handles)
    }

    fn to_masters_autodiscover() -> Masters {
        eprintln!("Ports autodiscovering is not implemented yet, pass parameter `--ports=simulated` for now");
        exit(1);
    }

    fn to_masters_ports(_ports: &[String]) -> Masters {
        eprintln!("Opening and discerning hardware ports is not implemented yet, pass parameter `--ports=simulated` for now");
        exit(1);
    }
}
