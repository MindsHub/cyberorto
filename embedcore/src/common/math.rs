use core::f32::consts::PI;
use micromath::F32Ext;

pub struct ComputedSin<const MICROSTEP: usize> {
    data: [f32; MICROSTEP],
}
impl<const MICROSTEP: usize> Default for ComputedSin<MICROSTEP> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const MICROSTEP: usize> ComputedSin<MICROSTEP> {
    pub fn new() -> Self {
        let mut data = [0.0f32; MICROSTEP];
        for (i, cur) in data.iter_mut().enumerate().take(MICROSTEP) {
            *cur = F32Ext::sin(PI / 2.0 / (MICROSTEP as f32) * (i as f32));
        }
        Self { data }
    }
    pub fn sin(&self, x: usize) -> f32 {
        let x = x % (MICROSTEP * 4);
        use core::cmp::Ordering::*;
        //cannot use match range statement with constant value
        if x < 2 * MICROSTEP {
            match x.cmp(&MICROSTEP) {
                Less => self.data[x],
                Equal => 1.0,
                Greater => self.data[2 * MICROSTEP - x],
            }
        } else {
            match x.cmp(&(MICROSTEP * 3)) {
                Less => -self.data[x - 2 * MICROSTEP],
                Equal => -1.0,
                Greater => -self.data[4 * MICROSTEP - x],
            }
        }
    }
    pub fn cos(&self, x: usize) -> f32 {
        self.sin(x + MICROSTEP)
    }
}

#[cfg(all(feature = "std", test))]

mod tests {
    extern crate std;
    use defmt_or_log::trace;

    use super::*;
    #[test]
    fn test_sin() {
        let m = ComputedSin::<5>::new();
        for i in 0..20 {
            let cur = F32Ext::sin(PI / 2.0 / (5 as f32) * (i as f32));
            trace!("{} {}", cur, m.sin(i));
            assert!((cur - m.sin(i)).abs() < 0.00002);
        }
    }
}
