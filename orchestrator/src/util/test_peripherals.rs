use std::time::Duration;

use embedcore::protocol::cyber::{DeviceIdentifier, Master, Message, Response};
use serialmessage::{ParseState, SerMsg};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::util::serial::SerialPorts;

const ID: u8 = 123;
const TIMEOUT: Duration = Duration::from_millis(100);
const RESEND_TIMES: u8 = 20;

pub async fn test_peripherals(ports: SerialPorts) {
    let ports = match ports {
        SerialPorts::Ports(items) => items,
        _ => SerialPorts::get_available_ports_or_exit(),
    };
    println!("Running tests on these serial ports: {ports:?}");

    for port in ports {
        println!();
        let Some(mut serial_port) = open_port(&port).await else { continue; };
        send_who_are_you_raw(&mut serial_port).await;
        receive_i_am_raw(&mut serial_port).await;
        let master = Master::new(serial_port, TIMEOUT, RESEND_TIMES);
        move_motor_with_master(&master).await;
    }
}

async fn open_port(port: &str) -> Option<tokio_serial::SerialStream> {
    println!("Opening serial port {port}...");
    let serial_port = tokio_serial::new(port, 115200)
        .timeout(TIMEOUT)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async();

    let serial_port = match serial_port {
        Ok(serial_port) => serial_port,
        Err(e) => {
            eprintln!("\x1b[31mError: Could not open port {port}: {e}\x1b[0m");
            return None;
        }
    };
    println!("\x1b[32mPort opened successfully\x1b[0m");

    Some(serial_port)
}

async fn send_who_are_you_raw(serial_port: &mut tokio_serial::SerialStream) {
    println!("Sending {:?} message with id {ID} manually", Message::WhoAreYou);
    let mut buf: [u8; 50] = [0; 50];
    let msg = match postcard::to_slice(&Message::WhoAreYou, &mut buf) {
        Ok(msg) => msg,
        Err(e) => {
            eprintln!("\x1b[31mpostcard::to_slice failed: {e}\x1b[0m");
            return;
        }
    };
    let Some((buf, len)) = SerMsg::create_msg_arr(msg, ID) else {
        eprintln!("\x1b[31mSerMsg::create_msg_arr failed\x1b[0m");
        return;
    };
    println!("{:?} bytes: {:?}", Message::WhoAreYou, &buf[0..len]);
    for b in &buf[0..len] {
        print!("{b} ");
        let res = tokio::time::timeout(
            TIMEOUT,
            tokio::io::AsyncWriteExt::write(serial_port, &[*b])
        ).await;
        match res {
            Ok(res) => {
                match res {
                    Ok(1) => {},
                    Ok(v) => {
                        eprintln!("\x1b[31mserial_port.write() should have written 1 byte, instead wrote {v}\x1b[0m");
                        break;
                    }
                    Err(e) => {
                        eprintln!("\x1b[31mserial_port.write() failed: {e}\x1b[0m");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("\x1b[31mserial_port.write() timeout: {e}\x1b[0m");
                break;
            }
        };
    }
    println!("\n\x1b[32mBytes sent successfully\x1b[0m");
}

async fn receive_i_am_raw(serial_port: &mut tokio_serial::SerialStream) {
    let sample_iam_message = {
        let mut buf: [u8; 50] = [0; 50];
        let msg = postcard::to_slice(&Response::Iam(DeviceIdentifier { name: *b"x         ", version: 1 }), &mut buf).unwrap();
        let (buf, len) = SerMsg::create_msg_arr(msg, ID).unwrap();
        buf[..len].to_vec()
    };
    println!("Receiving response to {:?} (should be Iam(...), e.g. {sample_iam_message:?})", Message::WhoAreYou);
    let mut input_buf = SerMsg::new();

    let res = tokio::time::timeout(
        TIMEOUT,
        async {
            let mut success = false;
            loop {
                let mut buf = [0u8];
                let b = match tokio::io::AsyncReadExt::read(serial_port, &mut buf).await {
                    Ok(1) => buf[0],
                    Ok(v) => {
                        eprintln!("\x1b[31mserial_port.read() should have read 1 byte, instead read {v}\x1b[0m");
                        break;
                    }
                    Err(e) => {
                        eprintln!("\x1b[31mserial_port.write() timeout: {e}\x1b[0m");
                        break;
                    }
                };
                print!("{b} ");

                let (state, _) = input_buf.parse_read_bytes(&[b]);
                if let ParseState::DataReady = state {
                    success = true;
                    break;
                }
            }
            success
        }
    ).await;

    match res {
        Ok(success) => {
            if !success {
                return;
            }
        }
        Err(e) => {
            eprintln!("\x1b[31mserial_port.read() timeout: {e}\x1b[0m");
            return;
        }
    };

    let id = input_buf.return_msg_id();
    let data = input_buf.return_read_data();
    println!("\nReceived message with id {id}: {data:?}");
    if id != ID {
        eprintln!("\x1b[31mMismatched IDs: expected {ID}, got {id}\x1b[0m");
    }

    let response = match postcard::from_bytes::<Response>(data) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("\x1b[31mCould not parse response: {e}\x1b[0m");
            return;
        }
    };

    println!("Received response: {response:?}");

    if let Response::Iam(iam) = response {
        println!("\x1b[32mReceived Iam successfully: {iam:?}\x1b[0m");
    } else {
        eprintln!("\x1b[31mDid not receive Iam as response\x1b[0m");
    }
}

async fn move_motor_with_master(master: &Master<SerialStream>) {
    println!("Sending move motor command using a Master");
    match master.move_to(100.0).await {
        Ok(_) => println!("\x1b[32mSent move motor command successfully\x1b[0m"),
        Err(e) => eprintln!("\x1b[31mCould not send move motor command: {e:?}\x1b[0m"),
    }
}
