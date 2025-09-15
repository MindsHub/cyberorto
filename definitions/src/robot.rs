use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RobotState {
    /// The current position of the end effector of the robot in meters
    /// (including any displacement caused by the currently selected tool):
    /// * `x`: in the direction of the rail, pointing from the reset position
    ///   to the other end of the rail
    /// * `y`: left and right from the rail, forms a right-handed system of coordinates
    /// * `z`: points up (the reset position is 0 so values will be mostly negative)
    pub position: Vec3,
    /// The current raw configuration of the motors in the joint space.
    /// * `x`: defined as [RobotState::position]'s `x` (but without tool displacements)
    /// * `y`: rotation of the tower in radians around the `z` axis pointing up
    /// * `z`: defined as [RobotState::position]'s `z` (but without tool displacements)
    pub position_joint: Vec3,
    /// The target position of the end effector of the robot, i.e. where the robot is
    /// trying to move to (see [RobotState::position] for a description of x, y, z).
    pub target: Vec3,
    /// The target raw configuration of the motors in the joint space, i.e. which
    /// configuration the joints are trying to go into (see
    /// [RobotState::position_config] for a description of x, y, z).
    pub target_joint: Vec3,
    /// How much water is in the tanks.
    pub water_level: WaterLevel,
    /// How much charge is in the battery.
    pub battery_level: BatteryLevel,
    /// The status of various connected actuators, e.g. whether they are on or off.
    pub actuators: Actuators,
    /// Whether there was an error connecting to any device.
    pub errors: Errors,
    /// Parameters, sizes and settings that determine how the robot is controlled.
    pub parameters: Parameters,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vec3 {
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
    /// How filled the water tank is as a value between 0.0 and 1.0.
    pub proportion: f32,
    /// How much water is in the tank, in liters.
    pub liters: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatteryLevel {
    /// How charged the battery is as a value between 0.0 and 1.0.
    pub proportion: f32,
    /// The battery voltage.
    pub volts: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Errors {
    /// Was there an error communicating to the x-axis motor?
    pub motor_x: Option<String>,
    /// Was there an error communicating to the y-axis motor?
    pub motor_y: Option<String>,
    /// Was there an error communicating to the z-axis motor?
    pub motor_z: Option<String>,
    /// Was there an error communicating to the embedded device handling actuators and sensors?
    pub peripherals: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    /// Length of the rotating arm, in meters.
    pub arm_length: f32,
    /// Length of the rail along which the tower moves, in meters.
    pub rail_length: f32,

    /// The battery voltage for when the battery is fully discharged.
    pub battery_voltage_min: f32,
    /// The battery voltage for when the battery is fully charged.
    pub battery_voltage_max: f32,

    /// The reading on the scale when the water tank is empty.
    pub water_scale_min: u32,
    /// The reading on the scale when the water tank is full.
    pub water_scale_max: u32,
    /// The capacity of the water tank in liters.
    pub water_tank_liters: f32,
}

/// DO NOT CHANGE THE VALUES HERE, they are just some sensible defaults for tests and for when
/// parameters could not be read. Change data in ~/.cyberorto/parameters.json instead!
impl Default for Parameters {
    fn default() -> Self {
        Self {
            arm_length: 1.511, // meters
            rail_length: 5.3, // meters
            battery_voltage_min: 12.0,
            battery_voltage_max: 13.5,
            water_scale_min: 8590000,
            water_scale_max: 9140000,
            water_tank_liters: 10.0,
        }
    }
}
