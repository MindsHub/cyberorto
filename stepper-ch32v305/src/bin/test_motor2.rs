#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(naked_functions)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]

use ch32_hal::usart::{Config, Uart};
use ch32v305::{driver, encoder, init, irqs};
use defmt_or_log::info;
use embassy_executor::*;
//type Motor<T> = DriverEncoderJoin<5, stepper::ch32::encoder::Encoder<'static>, Drv8843<'static, T, 5>>;

use embassy_time::{Instant, Timer};
use embedcore::common::motor::Motor;

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
    motor.align(2.0, 1.0).await;
    for s in 1..100 {
        let start = Instant::now();
        while start.elapsed().as_millis() < 1000 {
            //up to 20 steps
            motor.set_current(1.0).await;
            let wait = 1_000_000 / (20 * s);
            Timer::after_micros(wait).await;
        }
        info!("Speed: {}", s);
    }

    loop {Timer::after_secs(1).await;}
}
