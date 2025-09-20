#![cfg(test)]

use definitions::StepProgress;
use futures::future::BoxFuture;

use std::{thread::JoinHandle, time::Duration};

use super::*;
use crate::{
    action::{action_wrapper::Context, StepResult},
    state::tests::{get_test_state, TestState},
};

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

#[macro_export]
macro_rules! with_locked_queue {
    ($test_queue:ident, $locked_queue:ident, $content:block) => {{
        let $locked_queue = $test_queue.queue_handler.queue.0.lock().unwrap();
        $content
    }};
}

pub async fn test_with_queue(
    f: impl for<'a> Fn(&'a mut TestState, &'a mut TestQueue) -> BoxFuture<'a, ()>,
) {
    let (mut test_state, mut test_queue) = get_test_state_queue();

    // wait for queue to start up
    wait_for_nth_tick(&mut test_queue, 1, 0, 50).await;

    f(&mut test_state, &mut test_queue).await;

    test_queue.queue_handler.stop();
    let running_id = with_locked_queue!(test_queue, locked_queue, { locked_queue.running_id });
    if let Some(running_id) = running_id {
        test_queue
            .queue_handler
            .kill_running_action(running_id, false);
    }
    test_queue
        .queue_join_handle
        .join()
        .expect("Could not join queue");

    test_state
        .slave_bot_killer
        .send(())
        .expect("Could not send kill signal to slave bot");
    test_state
        .slave_bot_join_handle
        .join()
        .expect("Could not join slave bot");
}

#[macro_export]
macro_rules! test_with_queue {
    (async fn $test_name:ident ($state:ident: &mut TestState, $queue:ident: &mut TestQueue) $content:block ) => {
        #[tokio::test]
        async fn $test_name() {
            test_with_queue(|$state, $queue| async { $content }.boxed()).await;
        }
    };
}

pub async fn stop_queue_and_wait(q: &mut TestQueue, timeout_millis: usize) {
    q.queue_handler.stop();
    for _ in 0..timeout_millis {
        tokio::time::sleep(Duration::from_millis(1)).await;
        if q.queue_join_handle.is_finished() {
            return;
        }
    }
    panic!("Queue did not stop in time");
}

pub async fn wait_for_nth_tick(
    q: &mut TestQueue,
    min_wait_counter: usize,
    min_tick_counter: usize,
    timeout_millis: usize,
) {
    for _ in 0..timeout_millis {
        {
            let stats = q.queue_handler.test_stats.lock().unwrap();
            if stats.wait_counter >= min_wait_counter && stats.tick_counter >= min_tick_counter {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("Queue did not get to {min_wait_counter}th wait and {min_tick_counter}th tick in time");
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct InfiniteTestAction {
    pub i: u64,
}

#[async_trait]
impl Action for InfiniteTestAction {
    async fn step(&mut self, _: &Context, _: &StateHandler) -> StepResult {
        self.i += 1;
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.i += 1;
        StepResult::Running(StepProgress::Unknown)
    }

    fn get_type_name() -> &'static str {
        "infinite"
    }

    fn save_to_disk(&self, ctx: &Context) -> Result<(), String> {
        serialize_to_json_file(&self, &ctx.get_save_dir().join("data.json"))
    }

    fn load_from_disk(ctx: &Context) -> Result<Self, String> {
        deserialize_from_json_file(&ctx.get_save_dir().join("data.json"))
    }
}

#[derive(Debug, Default)]
pub struct StepResultTestAction {
    pub results: VecDeque<StepResult>,
}

#[async_trait]
impl Action for StepResultTestAction {
    async fn step(&mut self, _: &Context, _: &StateHandler) -> StepResult {
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.results.pop_front().unwrap_or(StepResult::Finished)
    }

    fn get_type_name() -> &'static str { "step_result" }
    fn save_to_disk(&self, _ctx: &Context) -> Result<(), String> { Ok(()) }
    fn load_from_disk(_ctx: &Context) -> Result<Self, String> { Ok(Self::default()) }
}
