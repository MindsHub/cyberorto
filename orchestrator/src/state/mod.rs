#![allow(unused_variables)] // TODO remove

pub(crate) mod tests;
pub mod dummy_message_handler;
mod kinematics;

use std::{
    sync::{Arc, Mutex, MutexGuard}, time::Duration
};

use definitions::{Parameters, RobotState, Vec3};
use embedcore::protocol::{communication::CommunicationError, cyber::Master};
use rocket::futures::future::{self, join4};
use tokio_serial::SerialStream;

use crate::{constants::{ARM_LENGTH, BATTERY_VOLTAGE_MAX, BATTERY_VOLTAGE_MIN, WATER_SCALE_MAX, WATER_SCALE_MIN, WATER_TANK_LITERS, WATER_TIME_MS}, state::kinematics::{joint_to_world, world_to_joint}, util::serial::Masters};

#[derive(Debug, Clone)]
pub struct Plant {
    x: f32,
    y: f32,
    z: f32,
}

type State = RobotState;

#[derive(Debug, Clone)]
pub struct StateHandler {
    state: Arc<Mutex<State>>,
    master_x: Arc<Master<SerialStream>>,
    master_y: Arc<Master<SerialStream>>,
    master_z: Arc<Master<SerialStream>>,
    /// Sensors might be implemented by a motor, so this may be a clone of one of
    /// master_x, master_y, master_z, so avoid using it while also using a motor!
    master_peripherals: Arc<Master<SerialStream>>,
}

fn acquire(state: &Arc<Mutex<State>>) -> MutexGuard<'_, State> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

macro_rules! mutate_state {
    ($state:expr, $($($field:ident).+ = $value:expr),+ $(,)?) => {
        {
            let mut state = acquire($state);
            $(state$(.$field)+ = $value;)*
        }
    };
}

impl StateHandler {
    pub fn new(masters: Masters) -> StateHandler {
        StateHandler {
            state: Arc::new(Mutex::new(State {
                // TODO read parameters from file
                parameters: Parameters {
                    arm_length: 1.511, // meters
                    rail_length: 5.3, // meters
                },
                ..Default::default()
            })),
            master_x: masters.x,
            master_y: masters.y,
            master_z: masters.z,
            master_peripherals: masters.peripherals,
        }
    }

    pub fn get_state(&self) -> State {
        acquire(&self.state).clone()
    }

    pub async fn water_a_plant(&self, x: f32, y: f32, z: f32) -> Result<(), StateHandlerError> {
        self.move_to(x, y, z).await?;
        self.water(WATER_TIME_MS).await?;
        tokio::time::sleep(Duration::from_millis(WATER_TIME_MS)).await;
        self.water(0).await?;
        Ok(())
    }

    pub fn add_plant(&self, x: f32, y: f32, z: f32) {
        // TODO add plant to DB
    }

    pub async fn water_all(&self) -> Result<(), StateHandlerError> {
        let plants: Vec<Plant> = vec![]; // TODO load plant from DB
        for plant in plants {
            self.water_a_plant(plant.x, plant.y, plant.z).await?;
        }
        Ok(())
    }

    pub async fn water(&self, cooldown_ms: u64) -> Result<(), StateHandlerError> {
        self.master_peripherals.water(cooldown_ms).await.map_err(StateHandlerError::Communication)
    }

    pub async fn lights(&self, cooldown_ms: u64) -> Result<(), StateHandlerError> {
        self.master_peripherals.lights(cooldown_ms).await.map_err(StateHandlerError::Communication)
    }

    pub async fn pump(&self, cooldown_ms: u64) -> Result<(), StateHandlerError> {
        self.master_peripherals.pump(cooldown_ms).await.map_err(StateHandlerError::Communication)
    }

    pub async fn plow(&self, cooldown_ms: u64) -> Result<(), StateHandlerError> {
        self.master_peripherals.plow(cooldown_ms).await.map_err(StateHandlerError::Communication)
    }

    pub async fn toggle_led(&self) -> Result<(), StateHandlerError> {
        let curr_led = self.master_peripherals.get_peripherals_state().await
            .map_err(StateHandlerError::Communication)?.led;
        self.master_peripherals.set_led(!curr_led).await.map_err(StateHandlerError::Communication)
    }

    pub async fn home(&self) -> Result<(), StateHandlerError> {
        self.move_to(0.0, 0.0, 0.0).await
    }

    pub async fn reset(&self) -> Result<(), StateHandlerError> {
        // first wait for the Z axis to reset
        self.retract().await?;

        // then also reset X and Y in parallel (to make things faster)
        mutate_state!(&self.state, target.x = 0.0, target.y = -ARM_LENGTH);
        let (res_x, res_y) = future::join(
            self.master_x.reset_motor(),
            self.master_y.reset_motor(),
        ).await;
        res_x.map_err(StateHandlerError::Communication)?;
        res_y.map_err(StateHandlerError::Communication)?;
        mutate_state!(&self.state, position.x = 0.0, position.y = -ARM_LENGTH);

        Ok(())
    }

    pub async fn retract(&self) -> Result<(), StateHandlerError> {
        // "retract" means resetting just the Z axis
        mutate_state!(&self.state, target.z = 0.0);
        self.master_z.reset_motor().await.map_err(StateHandlerError::Communication)?;
        mutate_state!(&self.state, position.z = 0.0);
        Ok(())
    }

    pub async fn move_to(&self, x: f32, y: f32, z: f32) -> Result<(), StateHandlerError> {
        let params = self.get_state().parameters.clone();
        let world = Vec3 { x, y, z };
        let Some(joint) = world_to_joint(&world, &params) else {
            return Err(StateHandlerError::InvalidWorldCoordinates(world));
        };

        // TODO compute trajectory that avoids obstacles and optimizes path
        // TODO handle errors while motors are moving and stop everything if errors happen
        mutate_state!(&self.state, target = world, target_joint = joint.clone());
        self.master_x.move_motor(joint.x).await.map_err(StateHandlerError::Communication)?;
        self.master_y.move_motor(joint.y).await.map_err(StateHandlerError::Communication)?;
        self.master_z.move_motor(joint.z).await.map_err(StateHandlerError::Communication)?;
        Ok(())
    }

    pub async fn try_update_state(&self) -> State {
        let (x, y, z, peripherals) = join4(
            self.master_x.get_motor_state(),
            self.master_y.get_motor_state(),
            self.master_z.get_motor_state(),
            self.master_peripherals.get_peripherals_state()
        ).await;

        let mut state = acquire(&self.state);

        match x {
            Ok(x) => state.position_joint.x = x.motor_pos,
            Err(_) => state.errors.motor_x = true,
        }
        match y {
            Ok(y) => state.position_joint.y = y.motor_pos,
            Err(_) => state.errors.motor_y = true,
        }
        match z {
            Ok(z) => state.position_joint.z = z.motor_pos,
            Err(_) => state.errors.motor_z = true,
        }
        state.position = joint_to_world(&state.position_joint, &state.parameters);

        match peripherals {
            Ok(peripherals) => {
                state.actuators.water = peripherals.water;
                state.actuators.lights = peripherals.lights;
                state.actuators.pump = peripherals.pump;
                state.actuators.plow = peripherals.plow;
                state.actuators.led = peripherals.led;
                state.water_level.proportion = (peripherals.water_scale - WATER_SCALE_MIN) as f32
                    / (WATER_SCALE_MAX - WATER_SCALE_MIN) as f32;
                state.water_level.liters = state.water_level.proportion * WATER_TANK_LITERS;
                state.battery_level.proportion = (peripherals.battery_voltage - BATTERY_VOLTAGE_MIN)
                    / (BATTERY_VOLTAGE_MAX - BATTERY_VOLTAGE_MIN);
                state.battery_level.volts = peripherals.battery_voltage;
            }
            Err(_) => state.errors.peripherals = true,
        }

        state.clone()
    }
}

#[derive(Debug)]
pub enum StateHandlerError {
    Communication(CommunicationError),
    InvalidWorldCoordinates(Vec3),
    GenericError(String),
}
