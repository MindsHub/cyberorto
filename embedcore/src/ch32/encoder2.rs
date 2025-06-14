use core::sync::atomic::Ordering;

use crate::{
    EncoderTrait,
    common::static_encoder::{ENCODER_VALUE, StaticEncoder},
};

use embassy_time::{Duration, Timer};
use embedded_hal::digital::InputPin;
pub struct EncoderPool2<Input: InputPin> {
    pins: (Input, Input),
    position: i32,
}

impl<Input: InputPin> EncoderPool2<Input> {
    pub fn new(p0: Input, p1: Input) -> Self {
        Self {
            pins: (p0, p1), //Mutex::new(Some((p0, p1, p2, p3))),
            position: 0,
        }
    }
    pub fn update(&mut self) {
        let prev_pos = self.position;
        let prev_phase = prev_pos.rem_euclid(4);
        let (p0, p1) = &mut self.pins;
        let to_check = (p0.is_high().unwrap_or(false), p1.is_high().unwrap_or(false));
        let cur_phase: i32 = match to_check {
            (true, true) => 0,
            (false, true) => 1,
            (false, false) => 2,
            (true, false) => 3,
        };

        if prev_phase == (1 + cur_phase) % 4 {
            self.position -= 1;
            return;
        }
        if cur_phase == (1 + prev_phase) % 4 {
            self.position += 1;
            return;
        }
    }
    /*pub async fn update_routine(&mut self) -> ! {
        let mut t = Ticker::every(Duration::from_hz(self.hz));
        loop {
            t.next().await;
            self.update();
        }
    }*/
    pub fn get_pos(&mut self) -> i32 {
        self.position
    }
}

impl<Input: InputPin> EncoderTrait for EncoderPool2<Input> {
    fn read(&mut self) -> i32 {
        self.update();
        self.get_pos()
    }
}

#[allow(async_fn_in_trait)]
pub trait GetStaticEncoderStd2: Sized + EncoderTrait {
    fn static_encoder(&self) -> StaticEncoder;
    #[inline(always)]
    async fn update_encoder(mut self, freq_hertz: u64) {
        loop {
            ENCODER_VALUE.store(self.read(), Ordering::Relaxed);
            Timer::after(Duration::from_hz(freq_hertz)).await;
        }
    }
}

impl<I: InputPin + 'static> GetStaticEncoderStd2 for EncoderPool2<I> {
    fn static_encoder(&self) -> StaticEncoder {
        StaticEncoder {
            direction: false,
            shift: 0,
        }
    }
}
