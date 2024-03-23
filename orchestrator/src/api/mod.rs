use crate::State;
use crate::queue::QueueHandler;

use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

mod from_request;

/******************************
* used structures definitions *
//region *********************/

#[derive(Serialize, Deserialize)]
pub struct Position {
  x: f32,
  y: f32,
  z: f32,
}

#[derive(Serialize, Deserialize)]
pub struct WaterLevel {
  percentage: f32,
  liters: f32,
}

#[derive(Serialize, Deserialize)]
pub struct BatteryLevel {
  percentage: f32,
  volts: f32,
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

// get position
#[get("/position")]
pub fn get_position(robot_state: State, queue_handler: &QueueHandler) -> Json<Position> {
    let position_data: Position = Position{
        x: 0.0, // TODO
        y: 0.0, // TODO
        z: 0.0, // TODO
    };

    return Json(position_data)
}

// get water level
#[get("/water-level")]
pub fn get_water_level(robot_state: State, queue_handler: &QueueHandler) -> Json<WaterLevel> {
    let water_level: WaterLevel = WaterLevel{
        percentage: 0.0, // TODO
        liters: 0.0, // TODO
    };

    return Json(water_level)
}

// get battery level
#[get("/battery-level")]
pub fn get_battery_level(robot_state: State, queue_handler: &QueueHandler) -> Json<BatteryLevel> {
    let battery_level: BatteryLevel = BatteryLevel{
        percentage: 0.0, // TODO
        volts: 0.0, // TODO
    };

    return Json(battery_level)
}
