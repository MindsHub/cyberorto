use std::{process::exit, sync::Arc, time::Duration};

use embedcore::{common::controllers::pid::CalibrationMode, protocol::cyber::{DeviceIdentifier, Master, Slave}};
use rocket::futures::{never::Never, executor::block_on};
use tokio::task::JoinHandle;
use tokio_serial::{SerialPortBuilderExt, SerialPortType, SerialStream};

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
            SerialPorts::Ports(ports) => (Self::to_masters_ports(ports, true), vec![]),
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
        let available_ports = match tokio_serial::available_ports() {
            Ok(available_ports) => available_ports,
            Err(e) => {
                eprintln!("Error: Could not obtain list of available serial ports: {e}");
                exit(1);
            }
        };

        let available_ports = available_ports.into_iter()
            .filter(|p| matches!(p.port_type, SerialPortType::UsbPort(_) | SerialPortType::Unknown))
            .map(|p| p.port_name)
            .collect::<Vec<String>>();

        if available_ports.is_empty() {
            eprintln!("Error: No serial ports discovered");
            exit(1);
        }

        Self::to_masters_ports(&available_ports, false)
    }

    fn to_masters_ports(ports: &[String], must_all_be_openable: bool) -> Masters {
        let mut x = None;
        let mut y = None;
        let mut z = None;
        let mut peripherals = None;

        fn set_var(
            var: &mut Option<Arc<Master<SerialStream>>>,
            capability: char,
            port: &str,
            master: &Arc<Master<SerialStream>>,
            id: &DeviceIdentifier,
        ) {
            if id.name.contains(&(capability as u8)) {
                if var.is_some() {
                    eprintln!("Error: Two serial devices say they can handle capability \"{capability}\", the last of which was {port}, whose identifier is {id:?}");
                    exit(1);
                }
                *var = Some(master.clone());
            }
        }

        fn assert_some(var: Option<Arc<Master<SerialStream>>>, capability: char) -> Arc<Master<SerialStream>> {
            match var {
                Some(var) => var,
                None => {
                    eprintln!("Error: No serial device can handle capability \"{capability}\"");
                    exit(1);
                },
            }
        }

        for port in ports {
            let serial_port = tokio_serial::new(port, 115200)
                .timeout(Duration::from_millis(3))
                .parity(tokio_serial::Parity::None)
                .stop_bits(tokio_serial::StopBits::One)
                .flow_control(tokio_serial::FlowControl::None)
                .open_native_async();

            let serial_port = match serial_port {
                Ok(serial_port) => serial_port,
                Err(e) => {
                    if must_all_be_openable {
                        eprintln!("Error: Could not open port {port}: {e}");
                        exit(1);
                    }
                    // probably this is not supposed to be a connected device, just ignore the error
                    continue;
                }
            };

            let master = Master::new(serial_port, 100000, 20);
            let id = match block_on(master.who_are_you()) {
                Ok(id) => id,
                Err(e) => {
                    if must_all_be_openable {
                        eprintln!("Error: Could not obtain device identifier from {port}: {e:?}");
                        exit(1);
                    }
                    // probably this is not supposed to be a connected device, just ignore the error
                    continue;
                }
            };

            let master = Arc::new(master);
            set_var(&mut x, 'x', port, &master, &id);
            set_var(&mut y, 'y', port, &master, &id);
            set_var(&mut z, 'z', port, &master, &id);
            set_var(&mut peripherals, 'p', port, &master, &id);
        }

        Masters {
            x: assert_some(x, 'x'),
            y: assert_some(y, 'y'),
            z: assert_some(z, 'z'),
            peripherals: assert_some(peripherals, 'p'),
        }
    }
}
