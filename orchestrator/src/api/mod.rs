use crate::{
    action::command_list::{Command, CommandListAction},
    queue::{QueueHandler, QueueState},
    state::{BatteryLevel, State, StateHandler, WaterLevel},
};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

mod from_request;

/******************************
* used structures definitions *
//region *********************/

// TODO: move struct in State and import it here
#[derive(Serialize, Deserialize)]
pub struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Devices {
    water: bool,
    lights: bool,
    pump: bool,
    plow: bool,
    led: bool,
}

#[derive(Serialize, Deserialize)]
pub struct RobotState {
    position: Position,
    target: Position,
    water_level: WaterLevel,
    battery_level: BatteryLevel,
    queue: QueueState,
    devices: Devices,
}

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
//endregion **********************/
/*
#[get("/")]
pub fn hello(robot_state: State, queue_handler: &QueueHandler) -> String {
    format!("Hello, {:?} {:?}!", robot_state, queue_handler)
}

// move xyz
#[post("/move")]
pub fn post_move(robot_state: State, queue_handler: &QueueHandler) {
    // TODO add action
}

// seed xyz
#[post("/seed")]
pub fn post_seed(robot_state: State, queue_handler: &QueueHandler) {
    // TODO add action
}

// water xyz
#[post("/water")]
pub fn post_water(robot_state: State, queue_handler: &QueueHandler) {
    // TODO add action
}*/

// get state
#[get("/state")]
pub async fn get_state(robot_state: &rocket::State<StateHandler>, queue: &QueueHandler) -> Result<Json<RobotState>, ()> {
    robot_state.update_state().await?;
    let state = robot_state.get_state();

    Ok(Json(RobotState {
        position: Position {
            x: state.x,
            y: state.y,
            z: state.z,
        },
        target: Position {
            x: state.target_x,
            y: state.target_y,
            z: state.target_z,
        },
        devices: Devices {
            water: state.water,
            lights: state.lights,
            pump: state.pump,
            plow: state.plow,
            led: state.led,
        },
        water_level: state.water_level,
        battery_level: state.battery_level,
        queue: queue.get_state(),
    }))
}

#[get("/toggle_led")]
pub async fn toggle_led(robot_state: &rocket::State<StateHandler>) -> Result<(), ()> {
    robot_state.toggle_led().await
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

#[post("/queue/kill-running-action", data = "<kill_running_action_args>")]
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
