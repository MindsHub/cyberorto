/// Code in this file was inspired by these bevy examples
/// https://bevy.org/examples/application/plugin/
/// https://gist.github.com/miketwenty1/baa1634fe558186e606c02932b8f37c8
use std::{path::PathBuf, time::Duration};

use async_channel::{Receiver, Sender};
use bevy::{app::{Plugin, Update}, ecs::{resource::Resource, schedule::IntoScheduleConfigs, system::{Res, ResMut}}, tasks::IoTaskPool, time::{common_conditions::on_timer, Time, Timer, TimerMode}};
use serde::{Deserialize, Serialize};

pub struct OrchestratorStateLoader {
    update_period: Duration,
    endpoint: String,
}

#[derive(Resource)]
struct OrchestratorStateLoaderRes {
    endpoint: String,
    tx: Sender<OrchestratorState>,
    rx: Receiver<OrchestratorState>,
}


#[derive(Serialize, Deserialize, Default, Resource)]
pub struct OrchestratorState {
    position: Position,
    target: Position,
    water_level: WaterLevel,
    battery_level: BatteryLevel,
    queue: QueueState,
    devices: Devices,
}

impl OrchestratorStateLoader {
    pub fn new(update_period: Duration, endpoint: String) -> Self {
        Self { update_period, endpoint }
    }
}

impl Plugin for OrchestratorStateLoader {
    fn build(&self, app: &mut bevy::app::App) {
        let (tx, rx) = async_channel::unbounded();
        app
            .insert_resource(
                OrchestratorStateLoaderRes {
                    endpoint: self.endpoint.clone(),
                    tx,
                    rx,
                }
            )
            .insert_resource(OrchestratorState::default())
            .add_systems(Update, update_from_downloaded)
            .add_systems(Update, download_from_orchestrator_if_needed.run_if(on_timer(self.update_period)));
    }
}

fn download_from_orchestrator_if_needed(
    mut res: ResMut<OrchestratorStateLoaderRes>,
    time: Res<Time>
) {
    let tx = res.tx.clone();
    let endpoint = res.endpoint.clone();

    IoTaskPool::get()
        .spawn(async move {
            let api_response_text = reqwest::get(format!("{endpoint}/state"))
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let _ = tx.try_send(api_response_text);
        })
        .detach();
}

fn update_from_downloaded(
    res: Res<OrchestratorStateLoaderRes>,
    mut state: ResMut<OrchestratorState>,
) {
    if let Ok(new_state) = res.rx.try_recv() {
        *state = new_state;
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Devices {
    water: bool,
    lights: bool,
    pump: bool,
    plow: bool,
    led: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WaterLevel {
    percentage: f32,
    liters: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryLevel {
    percentage: f32,
    volts: f32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct QueueState {
    paused: bool,
    stopped: bool,
    emergency: EmergencyStatus,
    save_dir: PathBuf,
    running_id: Option<ActionId>,
    actions: Vec<ActionInfo>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy, Default)]
enum EmergencyStatus {
    #[default]
    None,
    WaitingForReset,
    Resetting,
}

pub type ActionId = u32;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ActionInfo {
    id: ActionId,
    type_name: String,
    save_dir: PathBuf,
    is_running: bool,
}
