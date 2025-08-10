#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(naked_functions)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]

/*!
 * test
 * */
use ch32v305::init;
use embassy_executor::Spawner;
use embassy_time::{Instant, Timer};

use defmt_or_log::*;

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(_spawner: Spawner) -> ! {
    let _ = init();
    Timer::after_millis(1000).await;
    loop {
        let t = Instant::now();
        info!("test");
        info!("{}", t.elapsed());
    }
}
