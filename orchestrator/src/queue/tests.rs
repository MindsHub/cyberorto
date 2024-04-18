#![cfg(test)]

use std::{thread::{self, JoinHandle}, time::Duration};

use serialport::TTYPort;

use super::*;
use crate::state::{fake_slave_bot::FakeSlaveBotData, tests::TestState};

pub struct TestQueue {
    pub state_handler: StateHandler,
    pub slave_bot_join_handle: JoinHandle<()>,
    pub slave_bot_data: Arc<Mutex<FakeSlaveBotData>>,
    pub queue_handler: QueueHandler,
    pub save_dir: PathBuf,
    pub queue_join_handle: JoinHandle<()>,
}

impl Default for TestQueue {
    fn default() -> Self {
        let TestState {
            state_handler,
            slave_bot_join_handle,
            slave_bot_data,
        } = TestState::default();

        let save_dir = tempdir::TempDir::new("cyberorto_test").unwrap().into_path();
        let queue_handler = QueueHandler::new(state_handler.clone(), save_dir.clone());
        let queue_handler_clone = queue_handler.clone();
        let join_handle = std::thread::spawn(move || { queue_handler_clone.run() });

        TestQueue {
            state_handler,
            slave_bot_join_handle,
            slave_bot_data,
            queue_handler,
            save_dir,
            queue_join_handle: join_handle,
        }
    }
}

#[test]
fn queue_stops() {
    let TestQueue { queue_handler, queue_join_handle, .. } = TestQueue::default();

    queue_handler.stop();
    thread::sleep(Duration::from_millis(50));
    assert!(queue_join_handle.is_finished());
}
