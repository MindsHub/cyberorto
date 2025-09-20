#![cfg(test)]

use std::{assert_matches::assert_matches, fs};

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


test_with_queue!(
    async fn test_step_result_running(_s: &mut TestState, q: &mut TestQueue) {
        q.queue_handler.add_action(
            StepResultTestAction {
                results: vec![
                    StepResult::Running(StepProgress::Ratio { steps_done_so_far: 3, steps_total: 9 }),
                    StepResult::Running(StepProgress::Proportion(0.7)),
                    StepResult::Running(StepProgress::Unknown),
                    StepResult::Running(StepProgress::Count { steps_done_so_far: 12 }),
                ].into(),
            }
        );

        assert_matches!(q.queue_handler.get_state().actions[0].progress, StepProgress::Unknown);
        wait_for_nth_tick(q, 1, 2, 50).await;
        assert_matches!(q.queue_handler.get_state().actions[0].progress, StepProgress::Ratio { steps_done_so_far: 3, steps_total: 9 });
        wait_for_nth_tick(q, 1, 3, 50).await;
        assert_matches!(q.queue_handler.get_state().actions[0].progress, StepProgress::Proportion(0.7));
        wait_for_nth_tick(q, 1, 4, 50).await;
        // Unknown makes it so that the previous progress is kept!
        assert_matches!(q.queue_handler.get_state().actions[0].progress, StepProgress::Proportion(0.7));
        wait_for_nth_tick(q, 1, 5, 50).await;
        assert_matches!(q.queue_handler.get_state().actions[0].progress, StepProgress::Count { steps_done_so_far: 12 });
        wait_for_nth_tick(q, 2, 5, 50).await;
        with_locked_queue!(q, locked_queue, {
            assert!(!locked_queue.paused);
            assert_eq!(0, locked_queue.actions.len());
        });
    }
);

test_with_queue!(
    async fn test_step_result_running_error(_s: &mut TestState, q: &mut TestQueue) {
        q.queue_handler.add_action(
            StepResultTestAction {
                results: vec![
                    StepResult::RunningError(StateHandlerError::GenericError("whatever".into())),
                ].into(),
            }
        );

        wait_for_nth_tick(q, 2, 1, 50).await;
        // now the queue should have been paused because of the error
        with_locked_queue!(q, locked_queue, {
            assert!(locked_queue.paused);
            assert_eq!(1, locked_queue.actions.len());
            assert_matches!(locked_queue.actions[0].progress, StepProgress::Unknown);
            assert_matches!(locked_queue.actions[0].errors[0], StateHandlerError::GenericError(_));
            // make sure the action has been put back in the queue before pausing
            assert!(locked_queue.actions[0].action.is_some());
        });
    }
);

