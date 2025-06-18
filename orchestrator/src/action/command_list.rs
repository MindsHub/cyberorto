use std::{collections::VecDeque, future::Future, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    state::StateHandler,
    util::serde::{deserialize_from_json_file, serialize_to_json_file},
};

use super::{Action, Context};

/// Executes a list of commands directly on the robot state.
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandListAction {
    commands: VecDeque<Command>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Move { x: f32, y: f32, z: f32 },
    Reset,
    Home,
    Retract,
    Wait(Duration),
    WaterCooldown(Option<Duration>),
    WaterWait(Duration),
    LightsCooldown(Option<Duration>),
    LightsWait(Duration),
    PumpCooldown(Option<Duration>),
    PumpWait(Duration),
    PlowCooldown(Option<Duration>),
    PlowWait(Duration),
    ToggleLed,
}

impl CommandListAction {
    pub fn new(commands: Vec<Command>) -> Self {
        CommandListAction {
            commands: VecDeque::from(commands),
        }
    }

    async fn run_cooldown_function_for_duration<'a, F, Fut>(
        f: F,
        state_handler: &'a StateHandler,
        duration: Duration,
    ) -> Result<(), ()>
    where
        F: Fn(&'a StateHandler, Option<Duration>) -> Fut,
        Fut: Future<Output = Result<(), ()>>,
    {
        f(state_handler, Some(duration)).await?;
        tokio::time::sleep(duration).await;
        f(state_handler, None).await?;
        Ok(())
    }
}

#[async_trait]
impl Action for CommandListAction {
    async fn step(&mut self, _ctx: &Context, state_handler: &StateHandler) -> bool {
        let command = if let Some(command) = self.commands.pop_front() {
            command
        } else {
            return false;
        };

        let error = match command {
            Command::Move { x, y, z } => state_handler.move_to(x, y, z).await,
            Command::Reset => state_handler.reset().await,
            Command::Home => state_handler.home().await,
            Command::Retract => state_handler.retract().await,
            Command::Wait(duration) => {
                tokio::time::sleep(duration).await;
                Ok(())
            }
            Command::WaterCooldown(duration) => state_handler.water(duration).await,
            Command::WaterWait(duration) => {
                Self::run_cooldown_function_for_duration(StateHandler::water, state_handler, duration).await
            }
            Command::LightsCooldown(duration) => state_handler.lights(duration).await,
            Command::LightsWait(duration) => {
                Self::run_cooldown_function_for_duration(StateHandler::lights, state_handler, duration).await
            }
            Command::PumpCooldown(duration) => state_handler.pump(duration).await,
            Command::PumpWait(duration) => {
                Self::run_cooldown_function_for_duration(StateHandler::pump, state_handler, duration).await
            }
            Command::PlowCooldown(duration) => state_handler.plow(duration).await,
            Command::PlowWait(duration) => {
                Self::run_cooldown_function_for_duration(StateHandler::plow, state_handler, duration).await
            }
            Command::ToggleLed => state_handler.toggle_led().await,
        };

        if error.is_err() {
            println!("ERROR IN CommandListAction!");
        }

        !self.commands.is_empty()
    }

    fn get_type_name() -> &'static str
    where
        Self: Sized,
    {
        "command_list"
    }

    fn save_to_disk(&self, ctx: &Context) -> Result<(), String> {
        serialize_to_json_file(&self, &ctx.get_save_dir().join("data.json"))
    }

    fn load_from_disk(ctx: &Context) -> Result<Self, String>
    where
        Self: Sized,
    {
        deserialize_from_json_file(&ctx.get_save_dir().join("data.json"))
    }
}
