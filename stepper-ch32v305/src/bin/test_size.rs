#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case, unsafe_op_in_unsafe_fn, unused_imports, unused_mut)]

use ch32v305::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(_spawner: Spawner) -> ! {
    let _p = init();
    Timer::after_millis(100).await;

    //let mut e = create_encoder(p.PD1, p.PD2, p.PD3, p.PD4, &spawner);

    loop {
        Timer::after(Duration::from_millis(100)).await;
    }
    //x.write(buffer)
    //SerialWrapper::new(tx);
}
