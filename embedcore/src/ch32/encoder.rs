use core::sync::atomic::Ordering;

use crate::{
    EncoderTrait,
    common::static_encoder::{ENCODER_VALUE, StaticEncoder},
};

use embassy_time::{Duration, Timer};
use embedded_hal::digital::InputPin;
pub struct EncoderPool<Input: InputPin> {
    pins: (Input, Input, Input, Input),
    position: i32,
}

impl<Input: InputPin> EncoderPool<Input> {
    pub fn new(p0: Input, p1: Input, p2: Input, p3: Input) -> Self {
        Self {
            pins: (p0, p1, p2, p3), //Mutex::new(Some((p0, p1, p2, p3))),
            position: 0,
        }
    }
    pub fn update(&mut self) {
        let prev_pos = self.position;
        let prev_phase = prev_pos.rem_euclid(4);
        let (p0, p1, p2, p3) = &mut self.pins;
        let to_check = (
            p0.is_high().unwrap_or(false),
            p1.is_high().unwrap_or(false),
            p2.is_high().unwrap_or(false),
            p3.is_high().unwrap_or(false),
        );
        let cur_phase: i32 = match to_check {
            (true, true, false, false) => 0,
            (false, true, true, false) => 1,
            (false, false, true, true) => 2,
            (true, false, false, true) => 3,
            _ => {
                return;
            }
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

impl<Input: InputPin> EncoderTrait for EncoderPool<Input> {
    fn read(&mut self) -> i32 {
        self.update();
        self.get_pos()
    }
}

#[allow(async_fn_in_trait)]
pub trait GetStaticEncoderStd: Sized + EncoderTrait {
    fn static_encoder(&self) -> StaticEncoder;
    #[inline(always)]
    async fn update_encoder(mut self, freq_hertz: u64) {
        loop {
            ENCODER_VALUE.store(self.read(), Ordering::Relaxed);
            Timer::after(Duration::from_hz(freq_hertz)).await;
        }
    }
}

impl<I: InputPin + 'static> GetStaticEncoderStd for EncoderPool<I> {
    fn static_encoder(&self) -> StaticEncoder {
        StaticEncoder {
            direction: false,
            shift: 0,
        }
    }
}
