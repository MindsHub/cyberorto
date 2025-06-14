use core::time;
use std::time::Duration;

use embedcore::{common::{controllers::pid::{CalibrationMode, PidController}, motor::Motor}, protocol::cyber::{MessagesHandler, Response}, std::{get_fake_motor, FakeDriver, FakeEncoder}};
use tokio::time::Instant;


pub struct DummyMessageHandler {
    water_state: bool,
    lights_state: bool,
    pump_state: bool,
    plow_state: bool,
    led_state: bool,
    time_finished: Instant,
    motor: PidController<FakeEncoder, FakeDriver>,
}

impl DummyMessageHandler {
    pub fn new() -> DummyMessageHandler {
        Self { water_state: false, lights_state: false, pump_state: false, plow_state: false, led_state: false, time_finished: Instant::now(),
            motor: PidController::new(get_fake_motor(), 2.0, 2.0) }
    }
}

impl MessagesHandler for DummyMessageHandler {
    async fn move_motor(&mut self, x: f32) -> Option<Response> {
        self.motor.set_objective(((x * 1000.0) as i32));
        Some(Response::Done)
    }
    async fn reset_motor(&mut self) -> Option<Response> {
        self.motor.calibration(0, CalibrationMode::NoOvershoot);
        Some(Response::Done)
    }
    async fn poll(&mut self) -> Option<Response> {
        if self.time_finished <= Instant::now() {
            self.water_state = false;
            self.lights_state = false;
            self.pump_state = false;
            self.plow_state = false;
            Some(Response::Done)
        } else {
            Some(Response::Wait { ms: (self.time_finished - Instant::now()).as_millis() as u64 })
        }
    }
    async fn water(&mut self, ms: u64) -> Option<Response> {
        self.water_state = true;
        self.time_finished = Instant::now().checked_add(Duration::from_millis(ms))?;
        Some(Response::Done)
    }
    async fn lights(&mut self, ms: u64) -> Option<Response> {
        self.lights_state = true;
        self.time_finished = Instant::now().checked_add(Duration::from_millis(ms))?;
        Some(Response::Done)
    }
    async fn pump(&mut self, ms: u64) -> Option<Response> {
        self.pump_state = true;
        self.time_finished = Instant::now().checked_add(Duration::from_millis(ms))?;
        Some(Response::Done)
    }
    async fn plow(&mut self, ms: u64) -> Option<Response> {
        self.plow_state = true;
        self.time_finished = Instant::now().checked_add(Duration::from_millis(ms))?;
        Some(Response::Done)
    }
    async fn set_led(&mut self, state: bool) -> Option<Response> {
        self.led_state = state;
        Some(Response::Done)
    }
}