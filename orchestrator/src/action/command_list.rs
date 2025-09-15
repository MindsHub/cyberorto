use std::{future::Future, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    action::{StepProgress, StepResult}, state::{StateHandler, StateHandlerError}, util::serde::{deserialize_from_json_file, serialize_to_json_file}
};

use super::{Action, Context};

/// Executes a list of commands directly on the robot state.
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandListAction {
    commands: Vec<Command>,
    steps_done_so_far: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Move { x: f32, y: f32, z: f32 },
    Reset,
    Home,
    Retract,
    Wait(Duration),
    // TODO rename these, maybe to just Water and WaterManual?
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
            commands,
            steps_done_so_far: 0,
        }
    }

    fn duration_to_ms(duration: Duration) -> Result<u64, StateHandlerError> {
        duration.as_millis().try_into()
            .map_err(|e| StateHandlerError::GenericError(
                format!("Millis value for \"{duration:?}\" does not fit in u64: {e}")
            ))
    }

    fn option_duration_to_ms(duration: Option<Duration>) -> Result<u64, StateHandlerError> {
        duration
            .map(Self::duration_to_ms)
            // when no duration is provided, it means "turn off", i.e. 0 cooldown
            .unwrap_or(Ok(0))
    }

    async fn run_cooldown_function<'a, F, Fut>(
        f: F,
        state_handler: &'a StateHandler,
        duration: Option<Duration>,
    ) -> Result<(), StateHandlerError>
    where
        F: Fn(&'a StateHandler, u64) -> Fut,
        Fut: Future<Output = Result<(), StateHandlerError>>,
    {
        f(state_handler, Self::option_duration_to_ms(duration)?).await
    }

    async fn run_wait_function<'a, F, Fut>(
        f: F,
        state_handler: &'a StateHandler,
        duration: Duration,
    ) -> Result<(), StateHandlerError>
    where
        F: Fn(&'a StateHandler, u64) -> Fut,
        Fut: Future<Output = Result<(), StateHandlerError>>,
    {
        f(state_handler, Self::duration_to_ms(duration)?).await?;
        tokio::time::sleep(duration).await;
        f(state_handler, 0).await?;
        Ok(())
    }
}

#[async_trait]
impl Action for CommandListAction {
    async fn step(&mut self, _ctx: &Context, state_handler: &StateHandler) -> StepResult {
        if self.steps_done_so_far >= self.commands.len() {
            return StepResult::Finished;
        }
        let command = self.commands[self.steps_done_so_far].clone();

        let res = match command {
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
        };

        if let Err(e) = res {
            return StepResult::RunningError(e);
        }

        // only increment if there has been no error
        self.steps_done_so_far += 1;
        if self.steps_done_so_far >= self.commands.len() {
            StepResult::Finished
        } else {
            StepResult::Running(
                StepProgress::Ratio {
                    steps_done_so_far: self.steps_done_so_far,
                    steps_total: self.commands.len(),
                }
            )
        }
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
