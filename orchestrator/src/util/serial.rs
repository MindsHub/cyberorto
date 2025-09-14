use std::{path::Path, process::exit, sync::Arc, time::Duration};

use embedcore::{common::controllers::pid::CalibrationMode, protocol::cyber::{DeviceIdentifier, Master, Slave}};
use log::debug;
use rocket::futures::never::Never;
use tokio::task::JoinHandle;
use tokio_serial::{SerialPortBuilderExt, SerialPortType, SerialStream};

use crate::state::dummy_message_handler::DummyMessageHandler;


const TIMEOUT: Duration = Duration::from_millis(100);
const RESEND_TIMES: u8 = 20;


#[derive(Debug, Clone)]
pub enum SerialPorts {
    Simulated,
    Autodiscover,
    AutodiscoverOrSimulate,
    Ports(Vec<String>),
}

pub struct Masters {
    pub x: Arc<Master<SerialStream>>,
    pub y: Arc<Master<SerialStream>>,
    pub z: Arc<Master<SerialStream>>,
    pub peripherals: Arc<Master<SerialStream>>,
}

#[derive(Default)]
pub struct MastersOpt {
    pub x: Option<Arc<Master<SerialStream>>>,
    pub y: Option<Arc<Master<SerialStream>>>,
    pub z: Option<Arc<Master<SerialStream>>>,
    pub peripherals: Option<Arc<Master<SerialStream>>>,
}

impl SerialPorts {
    pub fn parse(s: &str) -> Result<SerialPorts, String> {
        if s == "simulated" {
            Ok(SerialPorts::Simulated)
        } else if s == "auto" {
            Ok(SerialPorts::Autodiscover)
        } else if s == "autosimulated" {
            Ok(SerialPorts::AutodiscoverOrSimulate)
        } else {
            Ok(SerialPorts::Ports(s.split(',').map(str::to_string).collect()))
        }
    }

    pub async fn to_masters(&self) -> (Masters, Vec<JoinHandle<Never>>) {
        match self {
            SerialPorts::Simulated => MastersOpt::default().into_masters_or_simulated(false),
            SerialPorts::Autodiscover => (Self::to_masters_autodiscover().await.into_masters(), vec![]),
            SerialPorts::AutodiscoverOrSimulate => Self::to_masters_autodiscover().await.into_masters_or_simulated(true),
            SerialPorts::Ports(ports) => (Self::to_masters_ports(ports, true).await.into_masters(), vec![]),
        }
    }

    pub fn get_available_ports_or_exit() -> Vec<String> {
        let available_ports = match tokio_serial::available_ports() {
            Ok(available_ports) => available_ports,
            Err(e) => {
                eprintln!("\x1b[31mError: Could not obtain list of available serial ports: {e}\x1b[0m");
                exit(1);
            }
        };

        let mut available_ports = available_ports.into_iter()
            .filter(|p| matches!(p.port_type, SerialPortType::UsbPort(_) | SerialPortType::Unknown))
            .map(|p| p.port_name)
            .collect::<Vec<String>>();

        // special path for the serial port exposed through pins on Raspberry,
        // which does not get reported by available_ports() for some reason
        if Path::new("/dev/serial0").exists() {
            available_ports.push("/dev/serial0".to_owned());
        }

        if available_ports.is_empty() {
            eprintln!("\x1b[31mError: No serial ports discovered\x1b[0m");
            exit(1);
        }

        available_ports
    }

    async fn to_masters_autodiscover() -> MastersOpt {
        let available_ports = Self::get_available_ports_or_exit();
        Self::to_masters_ports(&available_ports, false).await
    }

    async fn to_masters_ports(ports: &[String], must_all_be_openable: bool) -> MastersOpt {
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

        for port in ports {
            debug!("Opening serial port {port}...");
            let serial_port = tokio_serial::new(port, 115200)
                .timeout(TIMEOUT)
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
                    eprintln!("Warning: Could not open port {port}: {e}");
                    continue;
                }
            };
            eprintln!("Info: Opened port {port}");

            // let f = tokio::spawn(async move {
            //     for _ in 0..2000 {
            //         let a = tokio::io::AsyncWriteExt::write(&mut serial_port, &[1u8]).await;
            //         println!("res {a:?} {:?}", std::time::Instant::now());
            //         tokio::time::sleep(Duration::from_millis(1)).await;
            //     }
            //     println!("here {:?}", std::thread::current());
            //     5
            // });
            // println!("block on {:?}", block_on(f).unwrap());

            debug!("Opened serial port {port}, sending who_are_you()");
            let master = Master::new(serial_port, TIMEOUT, RESEND_TIMES);
            let id = match master.who_are_you().await {
                Ok(id) => id,
                Err(e) => {
                    if must_all_be_openable {
                        eprintln!("Error: Could not obtain device identifier from {port}: {e:?}");
                        exit(1);
                    }
                    // probably this is not supposed to be a connected device, just ignore the error
                    eprintln!("Warning: Could not obtain device identifier from {port}: {e:?}");
                    continue;
                }
            };
            eprintln!("Info: Obtained device identifier from port {port}: {id:?}");

            let master = Arc::new(master);
            set_var(&mut x, 'x', port, &master, &id);
            set_var(&mut y, 'y', port, &master, &id);
            set_var(&mut z, 'z', port, &master, &id);
            set_var(&mut peripherals, 'p', port, &master, &id);
        }

        MastersOpt { x, y, z, peripherals }
    }
}

impl MastersOpt {
    fn assert_some(var: Option<Arc<Master<SerialStream>>>, capability: char) -> Arc<Master<SerialStream>> {
        match var {
            Some(var) => var,
            None => {
                eprintln!("Error: No serial device can handle capability \"{capability}\"");
                exit(1);
            },
        }
    }

    fn into_masters(self) -> Masters {
        Masters {
            x: Self::assert_some(self.x, 'x'),
            y: Self::assert_some(self.y, 'y'),
            z: Self::assert_some(self.z, 'z'),
            peripherals: Self::assert_some(self.peripherals, 'p'),
        }
    }

    fn into_masters_or_simulated(self, require_at_least_one_real: bool) -> (Masters, Vec<JoinHandle<Never>>) {
        let mut masters = vec![];
        let mut motors = vec![];
        let mut join_handles = vec![];
        for (name, opt_master) in [
            (b"x         ", self.x),
            (b"y         ", self.y),
            (b"z         ", self.z),
            (b"p         ", self.peripherals),
        ] {
            if let Some(master) = opt_master {
                masters.push(master);
                // real Master exists for this struct, no need to simulate it
                continue;
            }

            let (master, slave) = SerialStream::pair()
                .expect("Failed to create dummy serial");
            masters.push(Arc::new(Master::new(master, TIMEOUT, RESEND_TIMES)));

            let (dummy_message_handler, motor) = DummyMessageHandler::new();
            let mut slave = Slave::new(slave, *name, dummy_message_handler);
            // TODO if the simulated serial hangs, the slave will not recover
            // (should not happen though).
            join_handles.push(tokio::task::spawn(async move { slave.run().await }));
            if name != b"p         " {
                // the last device is not a motor but a peripheral
                motors.push(motor);
            }
        }

        if require_at_least_one_real && join_handles.len() >= 4 {
            eprintln!("No real device connected found");
            exit(1);
        }

        assert_eq!(4, masters.len());
        //assert_eq!(3, motors.len()); -> only true if all things in self are None

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
}
