#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(naked_functions)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]

use ch32v305::init;
use embassy_executor::*;
//type Motor<T> = DriverEncoderJoin<5, stepper::ch32::encoder::Encoder<'static>, Drv8843<'static, T, 5>>;

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(_spawner: Spawner) -> ! {
    //init controller
    let _p = init();

    /*let pwm_pin1 = PwmPin::new_ch1::<2>(p.PA0);
    let pwm_pin2 = PwmPin::new_ch2::<2>(p.PA1);
    let pwm = SimplePwm::new(
        p.TIM2,
        Some(pwm_pin1),
        Some(pwm_pin2),
        None,
        None,
        Hertz::khz(10_000),
        CountingMode::default(),
    );

    let mut t = black_box(generate(black_box((p.PA4.degrade(), p.PA5.into(), p.PA2.into(), p.PA3.into())),
        black_box((p.PA10.degrade(), p.PA11.degrade(), p.PA12.degrade(), p.PB6.degrade(), p.PB7.degrade(), p.PA15.degrade(), pwm))
        ));


    //let c = mem::size_of::<DriverEncoderJoin<5, stepper::ch32::encoder::Encoder<'static>, Drv8843<'static, TIM2, 5>>>();
    loop{
        let start = Instant::now();
        let mut pos=0;
        for _ in 1..1000{
            pos+=black_box(t.read_update());
        }
        let time =  start.elapsed().as_micros();
    };*/
    todo!()
}
