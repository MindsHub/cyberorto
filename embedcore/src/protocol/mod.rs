//! Definitions for the comunication protocol used over serial
//! AsyncTrait is an abstraction over serial, it should be correctly implemented on each platform we support
//! In comunication mod there is the heavy lifting in order to implement any tipe of protocol
//! in cyber_protocol mod we define the protocol used between the motors and the raspberry

use core::future::Future;

pub mod comunication;
mod cyber_master;
mod cyber_protocol;
mod cyber_slave;
pub mod cyber {
    pub use super::cyber_master::Master;
    pub use super::cyber_protocol::*;
    pub use super::cyber_slave::Slave;
}

#[cfg(feature = "std")]
pub mod test_harness;
#[cfg(all(feature = "std", test))]
pub mod tests;
/// Serial abstraction. It's considered infallible
pub trait AsyncSerial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> impl Future<Output = u8>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> impl Future<Output = ()>;
}
