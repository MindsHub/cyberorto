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

    fn duration_to_ms(duration: Duration) -> Result<u64, ()> {
        duration.as_millis().try_into().map_err(|_| ())
    }

    fn option_duration_to_ms(duration: Option<Duration>) -> Result<u64, ()> {
        duration
            .map(Self::duration_to_ms)
            // when no duration is provided, it means "turn off", i.e. 0 cooldown
            .unwrap_or(Ok(0))
    }

    async fn run_cooldown_function<'a, F, Fut>(
        f: F,
        state_handler: &'a StateHandler,
        duration: Option<Duration>,
    ) -> Result<(), ()>
    where
        F: Fn(&'a StateHandler, u64) -> Fut,
        Fut: Future<Output = Result<(), ()>>,
    {
        f(state_handler, Self::option_duration_to_ms(duration)?).await
    }

    async fn run_wait_function<'a, F, Fut>(
        f: F,
        state_handler: &'a StateHandler,
        duration: Duration,
    ) -> Result<(), ()>
    where
        F: Fn(&'a StateHandler, u64) -> Fut,
        Fut: Future<Output = Result<(), ()>>,
    {
        f(state_handler, Self::duration_to_ms(duration)?).await?;
        tokio::time::sleep(duration).await;
        f(state_handler, 0).await?;
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

        match command {
            Command::Move { x, y, z } => state_handler.move_to(x, y, z).await,
            Command::Reset => state_handler.reset().await,
            Command::Home => state_handler.home().await,
            Command::Retract => state_handler.retract().await,
            Command::Wait(duration) => {
                tokio::time::sleep(duration).await;
                Ok(())
            }
            Command::WaterCooldown(duration) => {
                Self::run_cooldown_function(StateHandler::water, state_handler, duration).await
            }
            Command::WaterWait(duration) => {
                Self::run_wait_function(StateHandler::water, state_handler, duration).await
            }
            Command::LightsCooldown(duration) => {
                Self::run_cooldown_function(StateHandler::lights, state_handler, duration).await
            }
            Command::LightsWait(duration) => {
                Self::run_wait_function(StateHandler::lights, state_handler, duration).await
            }
            Command::PumpCooldown(duration) => {
                Self::run_cooldown_function(StateHandler::pump, state_handler, duration).await
            }
            Command::PumpWait(duration) => {
                Self::run_wait_function(StateHandler::pump, state_handler, duration).await
            }
            Command::PlowCooldown(duration) => {
                Self::run_cooldown_function(StateHandler::plow, state_handler, duration).await
            }
            Command::PlowWait(duration) => {
                Self::run_wait_function(StateHandler::plow, state_handler, duration).await
            }
            Command::ToggleLed => state_handler.toggle_led().await,
        }.unwrap(); // TODO handle errors

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
