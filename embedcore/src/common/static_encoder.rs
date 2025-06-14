use core::sync::atomic::Ordering;

use portable_atomic::AtomicI32;

use crate::EncoderTrait;

pub static ENCODER_VALUE: AtomicI32 = AtomicI32::new(0);

/// Usefull when there is only one encoder overall.
/// It is possible to clone and refer to it from multiple different places
pub struct StaticEncoder {
    pub direction: bool,
    pub shift: i32,
}

impl EncoderTrait for StaticEncoder {
    fn read(&mut self) -> i32 {
        let ret = ENCODER_VALUE.load(Ordering::Relaxed) + self.shift;
        if self.direction { -ret } else { ret }
    }
}
