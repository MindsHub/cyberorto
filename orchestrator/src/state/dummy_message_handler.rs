use std::{sync::Arc, time::Duration};

use embedcore::{
    common::controllers::pid::{CalibrationMode, PidController},
    protocol::cyber::{MessagesHandler, MotorState, PeripheralsState, Response},
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

    fn update_time_finished(var: &mut bool, time_finished: &mut Instant, cooldown_ms: u64) -> Response {
        if cooldown_ms == 0 {
            *var = false;
            return Response::Ok;
        }

        *var = true;
        match Instant::now().checked_add(Duration::from_millis(cooldown_ms)) {
            Some(t) => {
                *time_finished = t;
                Response::Ok
            }
            None => Response::Error(*b"duration +"),
        }
    }
}

impl MessagesHandler for DummyMessageHandler {
    async fn get_motor_state(&mut self) -> Response {
        let mut motor = self.motor.lock().await;
        Response::MotorState(MotorState {
            motor_pos: (motor.motor.read() as f32) / Self::METERS_TO_STEPS,
            is_idle: false, // TODO detect if idle
            error: None, // TODO maybe introduce errors sometimes?
        })
    }
    async fn reset_motor(&mut self) -> Response {
        // TODO implement better dummy reset logic
        self.motor.lock().await.set_objective(0);
        // self.motor
        //     .lock()
        //     .await
        //     .calibration(0, CalibrationMode::NoOvershoot)
        //     .await;
        Response::Ok
    }
    async fn move_motor(&mut self, x: f32) -> Response {
        self.motor.lock().await.set_objective((x * Self::METERS_TO_STEPS) as i32);
        Response::Ok
    }

    async fn get_peripherals_state(&mut self) -> Response {
        if self.time_finished <= Instant::now() {
            self.water_state = false;
            self.lights_state = false;
            self.pump_state = false;
            self.plow_state = false;
        }
        let resp_state = PeripheralsState {
            water: self.water_state,
            lights: self.lights_state,
            pump: self.pump_state,
            plow: self.plow_state,
            led: self.led_state,
            battery_voltage: rand::random_range(13.0..=13.4),
            water_scale: rand::random_range(8800000..=8900000),
        };
        // TODO add logging
        //println!("Got request for state: {resp_state:?}");
        Response::PeripheralsState(resp_state)
    }
    async fn water(&mut self, cooldown_ms: u64) -> Response {
        Self::update_time_finished(&mut self.water_state, &mut self.time_finished, cooldown_ms)
    }
    async fn lights(&mut self, cooldown_ms: u64) -> Response {
        Self::update_time_finished(&mut self.lights_state, &mut self.time_finished, cooldown_ms)
    }
    async fn pump(&mut self, cooldown_ms: u64) -> Response {
        Self::update_time_finished(&mut self.pump_state, &mut self.time_finished, cooldown_ms)
    }
    async fn plow(&mut self, cooldown_ms: u64) -> Response {
        Self::update_time_finished(&mut self.plow_state, &mut self.time_finished, cooldown_ms)
    }
    async fn set_led(&mut self, state: bool) -> Response {
        self.led_state = state;
        Response::Ok
    }
}
