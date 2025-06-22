use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RobotState {
    pub position: Position,
    pub target: Position,
    pub water_level: WaterLevel,
    pub battery_level: BatteryLevel,
    pub devices: Devices,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Devices {
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