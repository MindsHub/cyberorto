#![allow(unused_variables)] // TODO remove

pub(crate) mod tests;
pub mod dummy_message_handler;
mod kinematics;
pub mod parameters;

use std::{
    sync::{Arc, Mutex, MutexGuard}, time::Duration
};

use definitions::{Parameters, RobotState, Vec3};
use embedcore::protocol::{communication::CommunicationError, cyber::Master};
use rocket::futures::future::{self, join4};
use tokio_serial::SerialStream;

use crate::{state::kinematics::{joint_to_world, world_to_joint}, util::serial::Masters};

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
    motor_x: Arc<Master<SerialStream>>,
    motor_y: Arc<Master<SerialStream>>,
    motor_z: Arc<Master<SerialStream>>,
    peripherals: Arc<Master<SerialStream>>,
}

fn acquire(state: &Arc<Mutex<State>>) -> MutexGuard<'_, State> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

macro_rules! mutate_state {
    (&$self:ident.$state:ident, $($($field:ident).+ = $value:expr),+ $(,)?) => {
        {
            let mut $state = acquire(&$self.$state);
            $($state$(.$field)+ = $value;)*
        }
    };
}

macro_rules! handle_errors {
    ($self:ident.$master:ident.$func:ident ( $($args:expr)* )) => {
        async {
            let res = $self.$master.$func($($args)*).await;
            if let Err(e) = &res {
                mutate_state!(&$self.state, errors.$master = Some(format!("{e:?}")));
            }
            res.map_err(|e| StateHandlerError::Communication {
                error: e,
                device_name: stringify!($master),
                function_call: stringify!($func),
            })
        }
    };
}

impl StateHandler {
    pub fn new(masters: Masters, parameters: Parameters) -> StateHandler {
        StateHandler {
            state: Arc::new(Mutex::new(State {
                // TODO read parameters from file
                parameters,
                ..Default::default()
            })),
            motor_x: masters.x,
            motor_y: masters.y,
            motor_z: masters.z,
            peripherals: masters.peripherals,
        }
    }

    pub fn get_state(&self) -> State {
        acquire(&self.state).clone()
    }

    pub async fn water_a_plant(&self, x: f32, y: f32, z: f32) -> Result<(), StateHandlerError> {
        self.move_to(x, y, z).await?;
        // TODO allow passing amount of time
        self.water(5_000).await?;
        tokio::time::sleep(Duration::from_millis(5_000)).await;
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
        handle_errors!(self.peripherals.water(cooldown_ms)).await
    }

    pub async fn lights(&self, cooldown_ms: u64) -> Result<(), StateHandlerError> {
        handle_errors!(self.peripherals.lights(cooldown_ms)).await
    }

    pub async fn pump(&self, cooldown_ms: u64) -> Result<(), StateHandlerError> {
        handle_errors!(self.peripherals.pump(cooldown_ms)).await
    }

    pub async fn plow(&self, cooldown_ms: u64) -> Result<(), StateHandlerError> {
        handle_errors!(self.peripherals.plow(cooldown_ms)).await
    }

    pub async fn toggle_led(&self) -> Result<(), StateHandlerError> {
        let curr_led = handle_errors!(self.peripherals.get_peripherals_state()).await?.led;
        handle_errors!(self.peripherals.set_led(!curr_led)).await
    }

    pub async fn home(&self) -> Result<(), StateHandlerError> {
        self.move_to(0.0, 0.0, 0.0).await
    }

    pub async fn reset(&self) -> Result<(), StateHandlerError> {
        // first wait for the Z axis to reset
        self.retract().await?;

        // then also reset X and Y in parallel (to make things faster)
        mutate_state!(&self.state, target.x = 0.0, target.y = -state.parameters.arm_length);
        let (res_x, res_y) = future::join(
            handle_errors!(self.motor_x.reset_motor()),
            handle_errors!(self.motor_y.reset_motor()),
        ).await;
        res_x?;
        res_y?;
        mutate_state!(&self.state, position.x = 0.0, position.y = -state.parameters.arm_length);

        Ok(())
    }

    pub async fn retract(&self) -> Result<(), StateHandlerError> {
        // "retract" means resetting just the Z axis
        mutate_state!(&self.state, target.z = 0.0);
        handle_errors!(self.motor_z.reset_motor()).await?;
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
        handle_errors!(self.motor_x.move_motor(joint.x)).await?;
        handle_errors!(self.motor_y.move_motor(joint.y)).await?;
        handle_errors!(self.motor_z.move_motor(joint.z)).await?;
        Ok(())
    }

    pub async fn try_update_state(&self) -> State {
        let (x, y, z, peripherals) = join4(
            handle_errors!(self.motor_x.get_motor_state()),
            handle_errors!(self.motor_y.get_motor_state()),
            handle_errors!(self.motor_z.get_motor_state()),
            handle_errors!(self.peripherals.get_peripherals_state())
        ).await;

        let mut state = acquire(&self.state);

        if let Ok(x) = x {
            state.position_joint.x = x.motor_pos;
        }
        if let Ok(y) = y {
            state.position_joint.y = y.motor_pos;
        }
        if let Ok(z) = z {
            state.position_joint.z = z.motor_pos;
        }
        state.position = joint_to_world(&state.position_joint, &state.parameters);

        if let Ok(peripherals) = peripherals {
            state.actuators.water = peripherals.water;
            state.actuators.lights = peripherals.lights;
            state.actuators.pump = peripherals.pump;
            state.actuators.plow = peripherals.plow;
            state.actuators.led = peripherals.led;
            state.water_level.proportion = (peripherals.water_scale - state.parameters.water_scale_min) as f32
                / (state.parameters.water_scale_max - state.parameters.water_scale_min) as f32;
            state.water_level.liters = state.water_level.proportion * state.parameters.water_tank_liters;
            state.battery_level.proportion = (peripherals.battery_voltage - state.parameters.battery_voltage_min)
                / (state.parameters.battery_voltage_max - state.parameters.battery_voltage_min);
            state.battery_level.volts = peripherals.battery_voltage;
        }

        state.clone()
    }
}

#[derive(Debug)]
pub enum StateHandlerError {
    Communication {
        error: CommunicationError,
        device_name: &'static str,
        function_call: &'static str,
    },
    InvalidWorldCoordinates(Vec3),
    GenericError(String),
}
