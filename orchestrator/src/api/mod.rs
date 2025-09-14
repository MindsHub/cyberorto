use crate::{
    action::command_list::{Command, CommandListAction},
    queue::QueueHandler, state::StateHandler,
};
use definitions::RobotQueueState;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

mod from_request;

/******************************
* used structures definitions *
// region ********************/

#[derive(Serialize, Deserialize)]
pub struct KillRunningActionArgs {
    action_id: u32,
    keep_in_queue: bool,
}

#[derive(Serialize, Deserialize)]
pub struct KillRunningActionResponse {
    success: bool,
}

/**********************************
* end used structures definitions *
// endregion *********************/

// get state
#[get("/state")]
pub async fn get_state(robot_state: &StateHandler, queue: &QueueHandler) -> Result<Json<RobotQueueState>, ()> {
    // TODO add comment here and fix update_state errors
    Ok(Json(RobotQueueState { robot: robot_state.try_update_state().await, queue: queue.get_state() }))
}

#[get("/toggle_led")]
pub async fn toggle_led(robot_state: &StateHandler) -> Result<(), String> {
    robot_state.toggle_led().await.map_err(|e| format!("{e:?}"))
}

#[post("/queue/add_action_list", data = "<commands>")]
pub fn add_action_command_list(queue: &QueueHandler, commands: Json<Vec<Command>>) {
    queue.add_action(CommandListAction::new(commands.0));
}

#[post("/queue/pause")]
pub fn pause(queue: &QueueHandler) {
    queue.pause();
}

#[post("/queue/unpause")]
pub fn unpause(queue: &QueueHandler) {
    queue.unpause();
}

#[post("/queue/clear")]
pub fn clear(queue: &QueueHandler) {
    queue.clear();
}

#[post("/queue/kill_running_action", data = "<kill_running_action_args>")]
pub fn kill_running_action(
    queue: &QueueHandler,
    kill_running_action_args: Json<KillRunningActionArgs>,
) -> Json<KillRunningActionResponse> {
    let success = queue.kill_running_action(
        kill_running_action_args.action_id,
        kill_running_action_args.keep_in_queue,
    );
    Json(KillRunningActionResponse { success })
}
