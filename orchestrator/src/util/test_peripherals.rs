use crate::util::serial::SerialPorts;

pub async fn test_peripherals(ports: SerialPorts) {
    let ports = match ports {
        SerialPorts::Ports(items) => items,
        _ => SerialPorts::get_available_ports_or_exit(),
    };
    println!("Running tests on these serial ports: {ports:?}");

    for port in ports {
        println!("\nRunning tests on port {port}");
    }
}
