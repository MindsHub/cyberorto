use std::{sync::Arc, time::Duration};

use embedcore::{
    common::controllers::pid::{CalibrationMode, PidController},
    protocol::cyber::{MessagesHandler, Response, ResponseState},
    std::{get_fake_motor, FakeDriver, FakeEncoder},
    EncoderTrait,
};
use rocket::futures::lock::Mutex;
use tokio::time::Instant;

pub struct DummyMessageHandler {
    water_state: bool,
    lights_state: bool,
    pump_state: bool,
    plow_state: bool,
    led_state: bool,
    time_finished: Instant,
    motor: Arc<Mutex<PidController<FakeEncoder, FakeDriver>>>,
}

impl DummyMessageHandler {
    const METERS_TO_STEPS: f32 = 100000.0;

    pub fn new() -> (DummyMessageHandler, Arc<Mutex<PidController<FakeEncoder, FakeDriver>>>) {
        let motor = Arc::new(Mutex::new(PidController::new(get_fake_motor(), 2.0, 2.0)));
        (Self {
            water_state: false,
            lights_state: false,
            pump_state: false,
            plow_state: false,
            led_state: false,
            time_finished: Instant::now(),
            motor: motor.clone(),
        }, motor)
    }

    fn update_time_finished(var: &mut bool, time_finished: &mut Instant, cooldown_ms: u64) -> Option<Response> {
        if cooldown_ms == 0 {
            *var = false;
            return Some(Response::Done);
        }

        *var = true;
        match Instant::now().checked_add(Duration::from_millis(cooldown_ms)) {
            Some(t) => {
                *time_finished = t;
                Some(Response::Done)
            }
            None => Some(Response::Debug(*b"duration +")),
        }
    }
}

impl MessagesHandler for DummyMessageHandler {
    async fn move_motor(&mut self, x: f32) -> Option<Response> {
        self.motor.lock().await.set_objective((x * Self::METERS_TO_STEPS) as i32);
        Some(Response::Wait { ms: 100u64 })
    }
    async fn reset_motor(&mut self) -> Option<Response> {
        self.motor
            .lock()
            .await
            .calibration(0, CalibrationMode::NoOvershoot)
            .await;
        Some(Response::Done)
    }
    async fn poll(&mut self) -> Option<Response> {
        let difference = {
            let mut motor = self.motor.lock().await;
            motor.motor.read() as f32 - motor.pid.setpoint
        };
        if difference.abs() < 20.0 { // 20 steps (?)
            Some(Response::Done)
        } else {
            Some(Response::Wait { ms: 100u64 })
        }
    }
    async fn state(&mut self) -> Option<Response> {
        if self.time_finished <= Instant::now() {
            self.water_state = false;
            self.lights_state = false;
            self.pump_state = false;
            self.plow_state = false;
        }
        let resp_state = ResponseState {
            water: self.water_state,
            lights: self.lights_state,
            pump: self.pump_state,
            plow: self.plow_state,
            led: self.led_state,
            motor_pos: (self.motor.lock().await.motor.read() as f32) / Self::METERS_TO_STEPS,
        };
        // TODO add logging
        //println!("Got request for state: {resp_state:?}");
        Some(Response::State(resp_state))
    }
    async fn water(&mut self, cooldown_ms: u64) -> Option<Response> {
        Self::update_time_finished(&mut self.water_state, &mut self.time_finished, cooldown_ms)
    }
    async fn lights(&mut self, cooldown_ms: u64) -> Option<Response> {
        Self::update_time_finished(&mut self.lights_state, &mut self.time_finished, cooldown_ms)
    }
    async fn pump(&mut self, cooldown_ms: u64) -> Option<Response> {
        Self::update_time_finished(&mut self.pump_state, &mut self.time_finished, cooldown_ms)
    }
    async fn plow(&mut self, cooldown_ms: u64) -> Option<Response> {
        Self::update_time_finished(&mut self.plow_state, &mut self.time_finished, cooldown_ms)
    }
    async fn set_led(&mut self, state: bool) -> Option<Response> {
        self.led_state = state;
        Some(Response::Done)
    }
}
