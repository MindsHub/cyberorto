/*!
Std only implementations
*/
extern crate std;
use defmt_or_log::info;
use embassy_time::Instant;
use std::sync::mpsc::{Receiver, Sender, channel};
use tokio_serial::SerialStream;

use crate::{DiscreteDriver, EncoderTrait, common::motor::Motor, protocol::AsyncSerial};

/// implement AsyncSerial for SerialStream
impl AsyncSerial for SerialStream {
    async fn read(&mut self) -> u8 {
        let mut buf = [0u8];
        while tokio::io::AsyncReadExt::read(self, &mut buf).await.is_err() {}
        buf[0]
    }

    async fn write(&mut self, buf: u8) {
        while tokio::io::AsyncWriteExt::write(self, &[buf]).await.is_err() {}
        let _ = tokio::io::AsyncWriteExt::flush(self).await; // ignore the result
    }
}

pub struct FakeDriver {
    random_shift: u8,
    sender: Sender<(Instant, u8, f32)>,
}

pub struct FakeEncoder {
    last_update: Instant,
    position: f32,
    objective: f32,
    current: f32,
    cur_speed: f32,
    receiver: Receiver<(Instant, u8, f32)>,
}

pub fn get_fake_motor() -> Motor<FakeEncoder, FakeDriver> {
    let (sender, receiver) = channel();
    let driver = FakeDriver {
        random_shift: rand::random::<u8>() % 80,
        sender,
    };
    info!("random shift {}", driver.random_shift);
    let encoder = FakeEncoder {
        receiver,
        position: 0.0,
        cur_speed: 0.0,
        objective: 0.0,
        current: 0.0,
        last_update: Instant::now(),
    };

    Motor::new(encoder, driver, true)
}

impl DiscreteDriver for FakeDriver {
    const MICROSTEP: usize = 20;

    fn set_phase(&mut self, phase: u8, current: f32) {
        let _ = self
            .sender
            .send((Instant::now(), (phase + self.random_shift) % 80, current));
    }
}

impl FakeEncoder {
    /// compute according to (o-s(t) ) - d*s'(t)= s''(t)/a
    ///
    /// ## ASSUMPTIONS:
    /// - the motor energizes immediatly
    /// - in a unit of time it can move to a different solution
    /// - the acceleration is proportional to the difference between the current position and the objective, in reality it should be a sine wave
    ///
    /// ## DERIVATION
    ///
    /// that has solution (moving things around)
    /// s(t) = (c1*cos(at) + c2*sin(at))*e^(-dt) + o
    ///  
    /// c1=s0-o
    /// c2=(v0+d*c1)/a
    fn update(&mut self, i: Instant) {
        // a and decay_time are chosen experimentally
        // acceleration coefficient
        let a: f32 = 350.0 * self.current / 2.0;
        //after how much time the energy dissipated by the sistem is 1-e?
        let decay_time = 0.05;
        //compute d factor
        let d = 1.0 / decay_time;

        //elapsed time
        let time = (i - self.last_update).as_micros() as f32 / 1_000_000.0;

        // if no current, then we are not changing speed
        if self.current == 0.0 {
            self.position += time * self.cur_speed;
            return;
        }

        //coefficient computation
        let c1 = self.position - self.objective;
        let c2 = (self.cur_speed + d * c1) / a;

        //apply the formula
        let at = a * time;
        self.position =
            self.objective + (c2 * f32::sin(at) + c1 * f32::cos(at)) * f32::exp(-d * time);
        self.cur_speed = ((-d * c1 + c2 * a) * f32::cos(at) + (-d * c2 - a * c1) * f32::sin(at))
            * f32::exp(-d * time);

        // speed should never exeed 80_000
        debug_assert!(self.cur_speed.abs() < 80000.0);

        self.last_update = i;
    }
}
impl EncoderTrait for FakeEncoder {
    fn read(&mut self) -> i32 {
        while let Ok((instant, phase, current)) = self.receiver.try_recv() {
            self.update(instant);
            let mut delta = (phase as i32 - self.position.round() as i32).rem_euclid(80);
            if delta > 40 {
                delta -= 80;
            }
            assert!(delta.abs() <= 40);
            self.objective = self.position + delta as f32;
            self.current = current;
        }
        self.update(Instant::now());

        self.position.round() as i32
    }
}
#[cfg(test)]
mod test {
    use crate::{
        EncoderTrait,
        common::motor::test::{test_basic_movement, test_max_speed},
        std::get_fake_motor,
    };
    use test_log::test;
    extern crate std;
    #[test(tokio::test)]
    async fn test_align() {
        let mut m = get_fake_motor();
        m.align(1.0, 0.5).await;
        assert_eq!(m.read()%80, 0);
    }

    #[test(tokio::test)]
    async fn test_basic() {
        let mut m = get_fake_motor();
        test_basic_movement(&mut m, 2.0).await;
    }

    #[test(tokio::test)]
    async fn test_forward_speed() {
        let mut m = get_fake_motor();
        let forward = test_max_speed(&mut m, true).await;
        assert!(
            (9.9..15.0).contains(&forward),
            "invalid max speed {}",
            forward
        );
    }
    #[test(tokio::test)]
    async fn test_backward_speed() {
        let mut m = get_fake_motor();
        let backward = test_max_speed(&mut m, false).await;
        assert!(
            (-15.0..-9.9).contains(&backward),
            "invalid max speed {}",
            backward
        );
    }
}
