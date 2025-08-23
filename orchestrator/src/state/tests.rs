#![cfg(test)]

use embedcore::protocol::{cyber::{Message, Slave}, test_harness::{new_testable_slave, MessageRecorderSlave}};
use futures::future::{BoxFuture, FutureExt};
use tokio::sync::oneshot;

use std::thread::JoinHandle;

use super::*;

const FAKE_BOT_NAME: &[u8; 10] = b"test fake ";

pub struct TestState {
    pub state_handler: StateHandler,
    pub slave_bot_join_handle: JoinHandle<()>,
    pub slave_bot_killer: oneshot::Sender<()>,
    pub slave_bot_data: Arc<Mutex<MessageRecorderSlave>>,
}

pub fn get_test_state() -> TestState {
    let (master, slave) = SerialStream::pair().expect("Unable to create tty pair");

    let mut slave_bot: Slave<SerialStream, Arc<Mutex<MessageRecorderSlave>>> =
        new_testable_slave(slave, *FAKE_BOT_NAME);
    let slave_bot_data = slave_bot.message_handler.clone();

    let (slave_bot_killer_tx, slave_bot_killer_rx) = oneshot::channel();
    let slave_bot_join_handle = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                tokio::select! {
                    _ = slave_bot.run() => {}
                    _ = slave_bot_killer_rx => {}
                }
            });
    });

    let master = Arc::new(Master::new(master, Duration::from_millis(10), 8));
    TestState {
        state_handler: StateHandler::new(Masters { x: master.clone(), y: master.clone(), z: master.clone(), peripherals: master }),
        slave_bot_join_handle,
        slave_bot_killer: slave_bot_killer_tx,
        slave_bot_data,
    }
}

pub async fn test_with_state(f: impl Fn(&'_ mut TestState) -> BoxFuture<'_, ()>) {
    let mut test_state = get_test_state();
    f(&mut test_state).await;

    test_state
        .slave_bot_killer
        .send(())
        .expect("Could not send kill signal to slave bot");
    test_state
        .slave_bot_join_handle
        .join()
        .expect("Could not join slave bot");
}

macro_rules! test_with_state {
    (async fn $test_name:ident ($state:ident: &mut TestState) $content:block ) => {
        #[tokio::test]
        async fn $test_name() {
            test_with_state(|$state| async { $content }.boxed()).await;
        }
    };
}

test_with_state!(
    async fn test_toggle_led(s: &mut TestState) {
        let mut messages = Vec::new();
        for i in 0..10 {
            // TODO fix this test
            messages.push(Message::SetLed { led: i % 2 == 0 });
            s.state_handler.toggle_led().await.unwrap();
        }

        assert_eq!(messages, s.slave_bot_data.lock().unwrap().incoming,);
    }
);
