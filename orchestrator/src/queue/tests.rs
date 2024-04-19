#![cfg(test)]

use futures::future::{BoxFuture, FutureExt};

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

pub async fn test_with_queue(f: impl for<'a> Fn(&'a mut TestState, &'a mut TestQueue) -> BoxFuture<'a, ()>) {
    let (mut test_state, mut test_queue) = get_test_state_queue();
    f(&mut test_state, &mut test_queue).await;

    test_queue.queue_handler.stop();
    if let Some(running_id) = test_queue.queue_handler.queue.0.lock().unwrap().running_id {
        test_queue.queue_handler.kill_running_action(running_id, false);
    }
    test_queue.queue_join_handle.join().expect("Could not join queue");

    test_state
        .slave_bot_killer
        .send(())
        .expect("Could not send kill signal to slave bot");
    test_state
        .slave_bot_join_handle
        .join()
        .expect("Could not join slave bot");
}

macro_rules! test_with_queue {
    (async fn $test_name:ident ($state:ident: &mut TestState, $queue:ident: &mut TestQueue) $content:block ) => {
        #[tokio::test]
        async fn $test_name() {
            test_with_queue(|$state, $queue| async { $content }.boxed()).await;
        }
    };
}

test_with_queue!(async fn test_queue_stops(_s: &mut TestState, q: &mut TestQueue) {
    q.queue_handler.stop();
    thread::sleep(Duration::from_millis(50));
    assert!(q.queue_join_handle.is_finished());
});
