#![cfg(test)]

use super::*;

pub struct TestState {
    pub state_handler: StateHandler,
    pub slave: TTYPort,
}

impl Default for TestState {
    fn default() -> Self {
        let (mut master, slave) = TTYPort::pair().expect("Unable to create tty pair");
        TestState {
            state_handler: StateHandler::new(master),
            slave
        }
    }
}
