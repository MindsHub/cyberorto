use core::marker::PhantomData;

use ch32_hal::interrupt::typelevel::{self, Binding, Handler, Interrupt};
use ch32_hal::peripherals::{
    EXTI0, EXTI1, EXTI2, EXTI3, EXTI4, EXTI5, EXTI6, EXTI7, EXTI8, EXTI10, EXTI11, EXTI12, EXTI13,
    EXTI14, EXTI15,
};
//use ch32_hal::pac::Interrupt::{EXTI0, EXTI1};
use ch32_hal::{Peripheral, gpio::Pull};

use ch32_hal::gpio::{AnyPin, Input, Output, Pin as GpioPin};

use defmt_or_log::info;

use crate::common::static_encoder::{ENCODER_VALUE, StaticEncoder};

fn 𓀫() -> i32 {
    0
}
static mut EXTI_ENCODER_MANAGER: fn() -> i32 = 𓀫;
static mut ENCODER_PIN_1: Option<Input<'static>> = None;
static mut ENCODER_PIN_2: Option<Input<'static>> = None;
static mut LED_PIN: Option<Output<'static>> = None;

#[allow(dead_code)]
pub struct EncoderExti2<P1: GpioPin, P2: GpioPin, Irq>
where
    P1::ExtiChannel: EXT,
    P2::ExtiChannel: EXT,
    Irq: Binding<<P1::ExtiChannel as EXT>::Interrupt, RedirectExtiToEncoder<P1::ExtiChannel>>
        + Binding<<P2::ExtiChannel as EXT>::Interrupt, RedirectExtiToEncoder<P2::ExtiChannel>>,
{
    //p1: Input<'a>,
    //p2: Input<'a>,
    e1: P1::ExtiChannel,
    e2: P2::ExtiChannel,
    irq: PhantomData<Irq>,
}
#[allow(static_mut_refs)]
pub fn irqs() {
    let exti = &ch32_hal::pac::EXTI;

    let bits = exti.intfr().read();

    // We don't handle or change any EXTI lines above 24.
    let bits = bits.0 & 0x00FFFFFF;

    // Clear pending - Clears the EXTI's line pending bits.
    exti.intfr().write(|w| w.0 = bits);

    //exti.intenr().modify(|w| w.0 = w.0 & !bits);
    unsafe { LED_PIN.as_mut().map(|x| x.toggle()) };
}

impl<P1: GpioPin, P2: GpioPin, Irq> EncoderExti2<P1, P2, Irq>
where
    P1::ExtiChannel: EXT,
    P2::ExtiChannel: EXT,
    Irq: Binding<<P1::ExtiChannel as EXT>::Interrupt, RedirectExtiToEncoder<P1::ExtiChannel>>
        + Binding<<P2::ExtiChannel as EXT>::Interrupt, RedirectExtiToEncoder<P2::ExtiChannel>>,
{
    pub unsafe fn new(
        p1: P1,
        p2: P2,
        e1: P1::ExtiChannel,
        e2: P2::ExtiChannel,
        _: Irq,
        direction: bool,
        led: AnyPin,
    ) -> StaticEncoder {
        unsafe { <P1::ExtiChannel as EXT>::Interrupt::enable() };
        unsafe { <P2::ExtiChannel as EXT>::Interrupt::enable() };

        unsafe { enable_single(&p1) };
        unsafe { enable_single(&p2) };
        unsafe {
            LED_PIN = Some(Output::new(
                led,
                ch32_hal::gpio::Level::High,
                ch32_hal::gpio::Speed::High,
            ))
        };
        unsafe { ENCODER_PIN_1 = Some(Input::new(p1, Pull::Down)) };
        unsafe { ENCODER_PIN_2 = Some(Input::new(p2, Pull::Down)) };
        let _ = Self {
            e1,
            e2,
            irq: PhantomData,
        };
        unsafe { EXTI_ENCODER_MANAGER = Self::update };
        StaticEncoder { direction }
    }
    #[allow(static_mut_refs)]
    pub fn update() -> i32 {
        let exti = &ch32_hal::pac::EXTI;

        let bits = exti.intfr().read();

        // We don't handle or change any EXTI lines above 24.
        let bits = bits.0 & 0x00FFFFFF;

        // Clear pending - Clears the EXTI's line pending bits.
        exti.intfr().write(|w| w.0 = bits);

        unsafe { LED_PIN.as_mut().map(|x| x.toggle()) };
        let p1 = unsafe { ENCODER_PIN_1.as_ref().map(|x| x.is_high()).unwrap_or(false) };
        let p2 = unsafe { ENCODER_PIN_2.as_ref().unwrap().is_high() };
        let mut prev_pos = ENCODER_VALUE.load(core::sync::atomic::Ordering::Relaxed);
        let prev_phase = prev_pos.rem_euclid(4);

        let cur_phase = match (p1, p2) {
            (true, true) => 0,
            (false, true) => 1,
            (false, false) => 2,
            (true, false) => 3,
        };
        if prev_phase == (1 + cur_phase) % 4 {
            prev_pos -= 1;
        }
        if cur_phase == (1 + prev_phase) % 4 {
            prev_pos += 1;
        }
        ENCODER_VALUE.store(prev_pos, core::sync::atomic::Ordering::Relaxed);
        return prev_pos;
    }
}

pub struct RedirectExtiToEncoder<E1: EXT> {
    p: PhantomData<E1>,
}
#[allow(static_mut_refs)]
impl<E1: EXT> Handler<E1::Interrupt> for RedirectExtiToEncoder<E1> {
    unsafe fn on_interrupt() {
        unsafe {
            ENCODER_VALUE.store(
                EXTI_ENCODER_MANAGER(),
                core::sync::atomic::Ordering::Relaxed,
            )
        };
    }
}

/// make sure this struct has taken ownership of the correct EXTI, otherwise bad things will happen
unsafe fn enable_single<'a, T: GpioPin>(pin: &T) {
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

/*
critical_section::with(|_| {
                let exti = &crate::pac::EXTI;
                let afio = &crate::pac::AFIO;

                let port = port as u8;
                let pin = pin as usize;

                #[cfg(afio_v0)]
                {
                    // AFIO_EXTICR
                    // stride: 2, len: 15, 8 lines
                    afio.exticr().modify(|w| w.set_exti(pin, port));
                }
                // V1, V2, V3, L1
                #[cfg(any(afio_v3, afio_l1))]
                {
                    // AFIO_EXTICRx
                    // stride: 4, len: 4, 16 lines
                    afio.exticr(pin / 4).modify(|w| w.set_exti(pin % 4, port));
                }
                #[cfg(afio_x0)]
                {
                    // stride: 2, len: 15, 24 lines
                    afio.exticr(pin / 16).modify(|w| w.set_exti(pin % 16, port));
                }
                #[cfg(afio_ch641)]
                {
                    // single register
                    afio.exticr().modify(|w| w.set_exti(pin, port != 0));
                }

                // See-also: 7.4.3
                exti.intenr().modify(|w| w.set_mr(pin, true)); // enable interrupt

                exti.rtenr().modify(|w| w.set_tr(pin, rising));
                exti.ftenr().modify(|w| w.set_tr(pin, falling)); */

pub trait EXT: Peripheral {
    type Interrupt: Interrupt;
}
impl EXT for EXTI0 {
    type Interrupt = typelevel::EXTI0;
}
impl EXT for EXTI1 {
    type Interrupt = typelevel::EXTI1;
}
impl EXT for EXTI2 {
    type Interrupt = typelevel::EXTI2;
}
impl EXT for EXTI3 {
    type Interrupt = typelevel::EXTI3;
}
impl EXT for EXTI4 {
    type Interrupt = typelevel::EXTI4;
}
impl EXT for EXTI5 {
    type Interrupt = typelevel::EXTI9_5;
}
impl EXT for EXTI6 {
    type Interrupt = typelevel::EXTI9_5;
}
impl EXT for EXTI7 {
    type Interrupt = typelevel::EXTI9_5;
}
impl EXT for EXTI8 {
    type Interrupt = typelevel::EXTI9_5;
}
impl EXT for EXTI10 {
    type Interrupt = typelevel::EXTI15_10;
}
impl EXT for EXTI11 {
    type Interrupt = typelevel::EXTI15_10;
}

impl EXT for EXTI12 {
    type Interrupt = typelevel::EXTI15_10;
}

impl EXT for EXTI13 {
    type Interrupt = typelevel::EXTI15_10;
}

impl EXT for EXTI14 {
    type Interrupt = typelevel::EXTI15_10;
}

impl EXT for EXTI15 {
    type Interrupt = typelevel::EXTI15_10;
}
