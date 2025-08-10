#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case, unsafe_op_in_unsafe_fn, unused_imports, unused_mut)]

use embedcore::FlashTest;

use ch32v305::init;
use embassy_executor::Spawner;
use embassy_time::Timer;
use embedded_storage::{
    ReadStorage, Storage,
    nor_flash::{NorFlash, ReadNorFlash, RmwNorFlashStorage},
};

#[allow(dead_code)]
fn test_low_level(f: &mut FlashTest) {
    let mut buf = [0; 256];
    let _e = f.read(0, &mut buf);
    for (pos, val) in buf.iter_mut().enumerate() {
        *val = pos as u8;
    }
    let _e = f.erase(0, 256);
    let _e = f.write(0, &buf);

    let _e = f.read(0, &mut buf);
}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(_spawner: Spawner) -> ! {
    let _p = init();
    Timer::after_millis(100).await;

    let f = FlashTest::default();
    //test_low_level(&mut f);
    let mut buf = [0; 256];
    let mut t = RmwNorFlashStorage::new(f, &mut buf);
    let mut buf2 = [0; 256];
    t.read(0, &mut buf2).unwrap();
    t.write(256, &buf2).unwrap();
    t.read(256, &mut buf2).unwrap();
    todo!()
}
