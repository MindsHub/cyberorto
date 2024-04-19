#![cfg(test)]

use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use super::*;
use crate::state::tests::{get_test_state, TestState};

pub struct TestQueue {
    pub queue_handler: QueueHandler,
    pub save_dir: PathBuf,
    pub queue_join_handle: JoinHandle<()>,
}

pub fn get_test_state_queue() -> (TestState, TestQueue) {
    let test_state = get_test_state();

    let save_dir = tempdir::TempDir::new("cyberorto_test").unwrap().into_path();
    let queue_handler = QueueHandler::new(test_state.state_handler.clone(), save_dir.clone());
    let queue_handler_clone = queue_handler.clone();
    let queue_join_handle = std::thread::spawn(move || queue_handler_clone.run());

    (
        test_state,
        TestQueue {
            queue_handler,
            save_dir,
            queue_join_handle,
        },
    )
}

#[tokio::test]
async fn test_queue_stops() {
    let (_, q) = get_test_state_queue();

    q.queue_handler.stop();
    thread::sleep(Duration::from_millis(50));
    assert!(q.queue_join_handle.is_finished());
}
