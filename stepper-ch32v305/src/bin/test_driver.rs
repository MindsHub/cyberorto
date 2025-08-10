#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(naked_functions)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]

use ch32v305::{driver, encoder, init, irqs};
use defmt_or_log::info;
use embassy_executor::*;
use embassy_time::Timer;
use embedcore::DiscreteDriver;

irqs!();

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(_spawner: Spawner) -> ! {
    let p = init();

    let _e = encoder!(p, spawner, IrqsExti);
    let mut d = driver!(p, spawner);
    //let mut motor = Motor::new(e, d, true);
    //motor.set_phase(10, 0.5);
    for i in 0..400000i32 {
        //info!("pos:  {}", i);
        //d.low_level_current_set((i%20) as f32/20.0, 0.0);
        d.set_phase((i % 80) as u8, 0.5);
        Timer::after_micros(100).await;
    }
    d.set_phase(0, 0.0);
    //motor.set_phase(0, 0.0);
    /*info!("Motor initialized");

    let mut pwmb = SimplePwm::new(
        p.TIM8,
        None,
        None,
        Some(PwmPin::new_ch3::<0>(p.PC8)),
        None,
        Hertz::khz(100),
        CountingMode::EdgeAlignedUp,
    );
    pwmb.enable(timer::Channel::Ch3);
    let dira = Output::new(p.PB7, Level::Low, Speed::High);
    let dirb = Output::new(p.PC12, Level::High, Speed::High);
    let dirb = Output::new(p.PC11, Level::Low, Speed::High);
    let enable = Output::new(p.PB13, Level::High, Speed::High);

    pwmb.set_duty(timer::Channel::Ch3, 0);//pwmb.get_max_duty()
    Timer::after_millis(400).await;
    //motor.set_phase(0, 0.0);
    led.set_high();*/
    loop {Timer::after_secs(1).await;}
}
