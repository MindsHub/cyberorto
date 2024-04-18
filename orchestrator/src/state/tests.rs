#![cfg(test)]

use std::thread::JoinHandle;

use tokio::task::spawn_blocking;

use self::fake_slave_bot::{FakeSlaveBotData, FakeSlaveBot};

use super::*;

const FAKE_BOT_NAME: &[u8; 10] = b"test fake ";

pub struct TestState {
    pub state_handler: StateHandler,
    pub slave_bot_join_handle: JoinHandle<()>,
    pub slave_bot_data: Arc<Mutex<FakeSlaveBotData>>,
}

impl Default for TestState {
    fn default() -> Self {
        let (mut master, slave) = TTYPort::pair().expect("Unable to create tty pair");

        let mut slave_bot = FakeSlaveBot::new(slave, *FAKE_BOT_NAME);
        let slave_bot_data = slave_bot.get_data_ref();
        let slave_bot_join_handle = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    slave_bot.run().await
                });
        });

        TestState {
            state_handler: StateHandler::new(master),
            slave_bot_join_handle,
            slave_bot_data,
        }
    }
}

#[test]
fn test_toggle_led() {
    let TestState {
        state_handler,
        slave_bot_join_handle,
        slave_bot_data,
    } = TestState::default();

    state_handler.toggle_led();

    assert_eq!(
        vec![Message::SetLed { led: true }],
        slave_bot_data.lock().unwrap().received_messages,
    );
}