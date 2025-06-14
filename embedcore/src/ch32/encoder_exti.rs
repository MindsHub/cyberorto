/*use core::sync::atomic::Ordering;

use crate::{
    EncoderTrait,
    common::static_encoder::{ENCODER_VALUE, StaticEncoder},
};

use ch32_hal::exti::ExtiInput;

pub struct EncoderExti<'a> {
    pins: (ExtiInput<'a>, ExtiInput<'a>),
    position: i32,
}

impl<'a> EncoderExti<'a> {
    pub fn new(p0: ExtiInput<'a>, p1: ExtiInput<'a>) -> Self {
        Self {
            pins: (p0, p1), //Mutex::new(Some((p0, p1, p2, p3))),
            position: 0,
        }
    }
    ///function to continuisly call to get status update, the more updates the better
    /// it waits for a pinchange and then calculate what did change
    pub async fn update(&mut self) {
        let prev_pos = self.position;
        let prev_phase = prev_pos.rem_euclid(4);
        let (p0, p1) = &mut self.pins;
        let _ = embassy_futures::select::select(
            p0.wait_for_any_edge(),
            p1.wait_for_any_edge()
        )
        .await;

        let to_check = (p0.is_high(), p1.is_high(), );
        let cur_phase = match to_check {
            (true, true,) => 0,
            (false, true,) => 1,
            (false, false,) => 2,
            (true, false, ) => 3,
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
}

impl<'a> EncoderTrait for EncoderExti<'a> {
    fn read(&mut self) -> i32 {
        self.position
    }
}

#[allow(async_fn_in_trait)]
pub trait GetStaticEncoderExti: Sized + EncoderTrait {
    fn static_encoder(&self) -> StaticEncoder;
    async fn update_encoder(self);
}

impl<'a> GetStaticEncoderExti for EncoderExti<'a> {
    fn static_encoder(&self) -> StaticEncoder {
        StaticEncoder {
            direction: false,
            shift: 0,
        }
    }
    #[inline(always)]
    async fn update_encoder(mut self) {
        //let mut t = Ticker::every(Duration::from_hz(freq.0 as u64));
        loop {
            //trace!("update encoder");
            self.update().await;
            ENCODER_VALUE.store(self.read(), Ordering::Relaxed);
        }
    }
}
*/
