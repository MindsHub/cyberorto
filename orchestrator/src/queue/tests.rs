#![cfg(test)]

use std::fs;

use super::*;
use super::test_helpers::*;
use crate::{test_with_queue, with_locked_queue};

test_with_queue!(
    async fn test_stop(_s: &mut TestState, q: &mut TestQueue) {
        stop_queue_and_wait(q, 50).await;

        let saved = fs::read_to_string(q.save_dir.join("queue.json"))
            .expect("Queue did not save itself to disk");
        assert_eq!(r#"{"action_save_dirs":[],"id_counter":0}"#, saved);
    }
);

test_with_queue!(
    async fn test_stop_with_action(_s: &mut TestState, q: &mut TestQueue) {
        q.queue_handler.add_action(InfiniteTestAction::default());
        wait_for_nth_tick(q, 1, 1, 50).await;
        stop_queue_and_wait(q, 50).await;

        let action_dir = q.save_dir.join("0_infinite");
        let saved = fs::read_to_string(q.save_dir.join("queue.json"))
            .expect("Queue did not save itself to disk");
        assert_eq!(
            format!(
                r#"{{"action_save_dirs":[{:?}],"id_counter":1}}"#,
                action_dir
            ),
            saved
        );
        let saved = fs::read_to_string(action_dir.join("data.json"))
            .expect("Action did not save itself to disk");
        assert_eq!("{\"i\":2}", saved);
    }
);

test_with_queue!(
    async fn test_kill_action_keep_in_queue(_s: &mut TestState, q: &mut TestQueue) {
        let id = q.queue_handler.add_action(InfiniteTestAction::default());
        wait_for_nth_tick(q, 1, 1, 50).await;
        q.queue_handler
            .kill_running_action(id, /* keep_in_queue = */ true);
        stop_queue_and_wait(q, 50).await;

        let action_dir = q.save_dir.join("0_infinite");
        let saved = fs::read_to_string(q.save_dir.join("queue.json"))
            .expect("Queue did not save itself to disk");
        assert_eq!(
            format!(
                r#"{{"action_save_dirs":[{:?}],"id_counter":1}}"#,
                action_dir
            ),
            saved
        );
        let saved = fs::read_to_string(action_dir.join("data.json"))
            .expect("Action did not save itself to disk");
        assert_eq!("{\"i\":1}", saved);
    }
);

test_with_queue!(
    async fn test_kill_action_remove_from_queue(_s: &mut TestState, q: &mut TestQueue) {
        let id = q.queue_handler.add_action(InfiniteTestAction::default());
        wait_for_nth_tick(q, 1, 1, 50).await;
        q.queue_handler
            .kill_running_action(id, /* keep_in_queue = */ false);
        stop_queue_and_wait(q, 50).await;

        let action_dir = q.save_dir.join("0_infinite");
        let saved = fs::read_to_string(q.save_dir.join("queue.json"))
            .expect("Queue did not save itself to disk");
        assert_eq!(r#"{"action_save_dirs":[],"id_counter":1}"#, saved);
        fs::read_to_string(action_dir.join("data.json"))
            .expect_err("Action should not have been saved to disk");
    }
);

test_with_queue!(
    async fn test_pause(_s: &mut TestState, q: &mut TestQueue) {
        q.queue_handler.pause();
        wait_for_nth_tick(q, 2, 0, 50).await;
        with_locked_queue!(q, locked_queue, {
            assert!(locked_queue.paused);
        });

        q.queue_handler.add_action(InfiniteTestAction::default());
        wait_for_nth_tick(q, 3, 0, 50).await;
        with_locked_queue!(q, locked_queue, {
            assert!(locked_queue.paused);
            assert_eq!(1, locked_queue.actions.len());
            // make sure the action has not started executing
            assert!(locked_queue.actions[0].action.is_some());
        });

        q.queue_handler.unpause();
        wait_for_nth_tick(q, 3, 1, 50).await;
        with_locked_queue!(q, locked_queue, {
            assert!(!locked_queue.paused);
            assert_eq!(1, locked_queue.actions.len());
            // now the action has started executing
            assert!(locked_queue.actions[0].action.is_none());
        });
    }
);

test_with_queue!(
    async fn test_pause_during_action(_s: &mut TestState, q: &mut TestQueue) {
        q.queue_handler.add_action(InfiniteTestAction::default());
        wait_for_nth_tick(q, 1, 1, 50).await;
        with_locked_queue!(q, locked_queue, {
            assert!(!locked_queue.paused);
            assert_eq!(1, locked_queue.actions.len());
            // the action has started executing
            assert!(locked_queue.actions[0].action.is_none());
        });

        q.queue_handler.pause();
        wait_for_nth_tick(q, 2, 1, 50).await;
        with_locked_queue!(q, locked_queue, {
            assert!(locked_queue.paused);
            assert_eq!(1, locked_queue.actions.len());
            // make sure the action has been put back in the queue before pausing
            assert!(locked_queue.actions[0].action.is_some());
        });
    }
);
