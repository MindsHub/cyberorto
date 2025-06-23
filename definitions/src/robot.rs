use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RobotState {
    /// The current position of the end effector of the robot (including any
    /// displacement caused by the currently selected tool):
    /// * `x`: in the direction of the rail, pointing from the reset position
    ///   to the other end of the rail
    /// * `y`: left and right from the rail, forms a right-handed system of coordinates
    /// * `z`: points up (the reset position is 0 so values will be mostly negative)
    pub position: Position,
    /// The current raw configuration of the motors.
    /// * `x`: defined as [RobotState::position]'s `x` (but without tool displacements)
    /// * `y`: rotation of the tower in radians around the `z` axis pointing up
    /// * `z`: defined as [RobotState::position]'s `z` (but without tool displacements)
    pub position_config: Position,
    /// The target position of the end effector of the robot, i.e. where the robot is
    /// trying to move to (see [RobotState::position] for a description of x, y, z).
    pub target: Position,
    /// The target raw configuration of the motors, i.e. which configuration the robot is
    /// trying to go into (see [RobotState::position_config] for a description of x, y, z).
    pub target_config: Position,
    /// How much water is in the tanks.
    pub water_level: WaterLevel,
    /// How much charge is in the battery.
    pub battery_level: BatteryLevel,
    /// The status of various connected actuators, e.g. whether they are on or off.
    pub actuators: Actuators,
    /// Whether there was an error connecting to any device
    pub errors: Errors,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Actuators {
    pub water: bool,
    pub lights: bool,
    pub pump: bool,
    pub plow: bool,
    pub led: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WaterLevel {
    pub percentage: f32,
    pub liters: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatteryLevel {
    pub percentage: f32,
    pub volts: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Errors {
    /// Was there an error communicating to the x-axis motor?
    pub motor_x: bool,
    /// Was there an error communicating to the y-axis motor?
    pub motor_y: bool,
    /// Was there an error communicating to the z-axis motor?
    pub motor_z: bool,
    /// Was there an error communicating to the embedded device handling actuators and sensors?
    pub peripherals: bool,
}
