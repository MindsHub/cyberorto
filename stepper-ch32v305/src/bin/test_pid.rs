#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case, unsafe_op_in_unsafe_fn, unused_imports, unused_mut)]
use ch32_hal::gpio::{AnyPin, Level, Output, Pin, Speed};
use ch32v305::*;
use defmt_or_log::info;
use embassy_executor::*;
use embassy_time::{Instant, Timer};
use embedcore::{
    common::{
        controllers::pid::{CalibrationMode, PidController}, motor::Motor
    }, EncoderTrait
};
irqs!();

#[embassy_executor::task]
async fn led(l: AnyPin) {
    let mut l = Output::new(l, Level::Low, Speed::High);
    loop {
        l.set_high();
        Timer::after_millis(100).await;
        l.set_low();
        Timer::after_millis(100).await;
    }
}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let p = init();
    let e = encoder!(p, spawner, IrqsExti);
    spawner.must_spawn(led(p.PA4.degrade()));
    let d = driver!(p, spawner);
    let m = Motor::new(e, d, false);
    // wait 100 ms to wait debug connection to come live
    Timer::after_millis(500).await;

    let mut pid = PidController::new(m, 2.0, 2.0);
    //pid.motor.align(1.0, 1.0).await;

    let _ = pid.calibration(2000, CalibrationMode::NoOvershoot).await;
    info!("Motor initialized");

    loop {
        info!("{}", pid.motor.read());

        pid.set_objective(10000);
        let t: Instant = Instant::now();
        while t.elapsed().as_millis() < 5000 {
            pid.update().await;
            embassy_futures::yield_now().await;
        }
        let t = Instant::now();
        pid.set_objective(-10000);
        while t.elapsed().as_millis() < 5000 {
            pid.update().await;
            embassy_futures::yield_now().await;
        }
    }
}
