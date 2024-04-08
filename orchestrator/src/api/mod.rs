use crate::queue::QueueHandler;
use crate::state::{State, WaterLevel, BatteryLevel};

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
pub struct RobotState {
    position:      Position,
    water_level:   WaterLevel,
    battery_level: BatteryLevel,
}

/**********************************
* end used structures definitions *
//endregion **********************/

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
}

// get state
#[get("/state")]
pub fn get_state(robot_state: State) -> Json<RobotState> {

    let position_data: RobotState = RobotState {
        position: Position {
            x: robot_state.x,
            y: robot_state.y,
            z: robot_state.z,
        },
        water_level:   robot_state.water_level,
        battery_level: robot_state.battery_level,
    };

    return Json(position_data);
}
