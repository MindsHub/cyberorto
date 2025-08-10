#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case, unsafe_op_in_unsafe_fn, unused_imports, unused_mut)]
use ch32_hal::gpio::Pin;
use ch32v305::{init, irqs};
use defmt_or_log::info;
use embassy_executor::*;
use embassy_time::Timer;
use embedcore::{EncoderExti2, EncoderTrait};

irqs!();
/*unsafe fn enable_single<'a, T: Pin>(pin: &T) {
    let port = pin.port() as u8;
    let pin_id = pin.pin() as usize;
    critical_section::with(|_| {
        let exti = &ch32_hal::pac::EXTI;
        let afio = &ch32_hal::pac::AFIO;
        //into_ref!(pin, ch);

        //TODO WARNING: ONLY ON V3
        // AFIO_EXTICRx
        // stride: 4, len: 4, 16 lines

        afio.exticr(pin_id / 4)
            .modify(|w| w.set_exti(pin_id % 4, port));

        exti.intenr().modify(|w| w.set_mr(pin_id, true)); // enable interrupt

        //enable on falling and rising
        exti.rtenr().modify(|w| w.set_tr(pin_id, true));
        exti.ftenr().modify(|w| w.set_tr(pin_id, true));

        // set pull mode
    });
    info!("Enabled EXTI {} {}", port, pin_id);
}

static mut LED_PIN: Option<Output<'static>> = None;
#[allow(static_mut_refs)]
fn irqs() {
    let exti = &ch32_hal::pac::EXTI;

    let bits = exti.intfr().read();

    // We don't handle or change any EXTI lines above 24.
    let bits = bits.0 & 0x00FFFFFF;

    // Clear pending - Clears the EXTI's line pending bits.
    exti.intfr().write(|w| w.0 = bits);

    //exti.intenr().modify(|w| w.0 = w.0 & !bits);
    unsafe { LED_PIN.as_mut().map(|x| x.toggle()) };
}

#[interrupt]
unsafe fn EXTI9_5() {
    irqs();
}
#[interrupt]
unsafe fn EXTI15_10() {
    irqs();
}

fn enable_interrupt() {
    use ch32_hal::pac::Interrupt;
    unsafe { qingke::pfic::enable_interrupt(Interrupt::EXTI9_5 as u8) };
    unsafe { qingke::pfic::enable_interrupt(Interrupt::EXTI15_10 as u8) };
}*/

#[embassy_executor::main(entry = "qingke_rt::entry")]
#[highcode]
async fn main(_spawner: Spawner) -> ! {
    let p = init();
    Timer::after_millis(500).await;
    /*let led = Output::new(
         p.PA4,
         ch32_hal::gpio::Level::High,
         ch32_hal::gpio::Speed::High,
     );
    unsafe { LED_PIN = Some(led) };
     enable_interrupt();
     unsafe {
         enable_single(&p.PA5);
         enable_single(&p.PB10)
     };
     let pa5 = Input::new(p.PA5, ch32_hal::gpio::Pull::Up);
     let pb10 = Input::new(p.PB10, ch32_hal::gpio::Pull::Up);*/

    let mut e = unsafe {
        EncoderExti2::new(
            p.PA5,
            p.PB10,
            p.EXTI5,
            p.EXTI10,
            IrqsExti,
            false,
            p.PA4.degrade(),
        )
    };
    //init controller

    //let mut e = encoder!(p, spawner);

    info!("init done");

    loop {
        //irqs();
        //EncoderExti2::<PA7, PC11, Irqs>::update();
        let dummy = e.read();
        info!("pos:  {}", dummy);
        //info!("pos:  {} {}", pa5.is_high(), pb10.is_high());

        Timer::after_millis(1000).await;
    }
}
