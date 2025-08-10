#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(naked_functions)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
use ch32_hal::usart::{Config, Uart};
use ch32v305::{driver, encoder, init, irqs};
use defmt_or_log::debug;
use embassy_executor::*;
//type Motor<T> = DriverEncoderJoin<5, stepper::ch32::encoder::Encoder<'static>, Drv8843<'static, T, 5>>;

use embassy_time::Timer;
use embedcore::common::motor::{
    Motor,
    test::{test_basic_movement, test_max_speed},
};

irqs!();

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(_spawner: Spawner) -> ! {
    //init controller
    let p = init();
    //serial_port: USART1 PA8 PB15
    let _u = Uart::new(
        p.USART1,
        p.PA8,
        p.PB15,
        IrqsUsart,
        p.DMA1_CH4,
        p.DMA1_CH5,
        Config::default(),
    );
    let e = encoder!(p, spawner, IrqsExti);
    let d = driver!(p, spawner);

    let mut motor = Motor::new(e, d, true);
    //motor.set_phase(10, 2.0);
    //Timer::after_secs(200).await;
    for _ in 0..1 {
        test_basic_movement(&mut motor, 2.0).await;
    }
    Timer::after_millis(100).await;
    for _ in 0..100 {
        let forward = test_max_speed(&mut motor, true).await;
        defmt_or_log::debug!("{}", forward);
        Timer::after_millis(100).await;
        let backward = test_max_speed(&mut motor, false).await;
        defmt_or_log::debug!("{}", backward);
        Timer::after_millis(100).await;
        debug!("speeds: {} {}", forward, backward);
    }

    loop {Timer::after_secs(1).await;}
}
