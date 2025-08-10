#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(naked_functions)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]
/*!
 *  This module maps generic, or generic ch32 structs to a physical pin. This represent an hardware implementation.
 * Then there is an init function that permits to init all thats needed by each binary with ease
 *
 *
 * The mapping of pin to function is:
 * debug probe: PA13, PA14
 * serial_port: USART1 PA8 PB15
 * encoder:  PA5 PB10
 *
 * driver pwm(A, B): TIM3 PC7(2) PC8(3)
 * driver_dir(A, B): PB6 PB7
 * driver reset fault decay: PB13 PB14 PC6
 * */

#[cfg(feature = "defmt")]
mod defmt_impl;
mod driver;
pub mod encoder;
pub mod irqs;
use ch32_hal::rcc::{Pll, PllMul, PllPreDiv, PllSource, Sysclk};
#[cfg(feature = "defmt")]
use defmt_impl::SDIPrint;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt_or_log::error!("\n\n\n{}", info);

    loop {}
}

use ch32_hal::interrupt::typelevel::Binding;
use ch32_hal::peripherals::{DMA1_CH4, DMA1_CH5, PA8, PB15, USART1};
use ch32_hal::usart::{self, Uart};
use ch32_hal::{self as hal, Config};
use ch32_hal::{Peripherals, rcc};

use embedcore::SerialWrapper;

pub fn init() -> Peripherals {
    // IMPORTANT COMMENT: using the default clock (i.e. rcc::Config::default()) results in broken
    // asyncs and various other strange shenanigans. Using SYSCLK_FREQ_144MHZ_HSI makes everything
    // work except for the baudrate calculation for serial ports, where the baudrate MUST be scaled
    // by `82/68` before passing it to `Uart`. E.g. 115200 turns into 138917.
    let config = ch32_hal::Config {
        rcc: rcc::Config::SYSCLK_FREQ_144MHZ_HSI,
        ..Default::default()
    };
    #[cfg(feature = "defmt")]
    SDIPrint::enable();
    let p = hal::init(config);
    unsafe {
        hal::embassy::init();
    }
    p
}

// TODO implement serial
/*pub fn serial<
    Irqs: 'static + Binding<ch32_hal::interrupt::typelevel::USART1, usart::InterruptHandler<USART1>>,
>(
    usart: USART1,
    rx: PA8,
    tx: PB15,
    irqs: Irqs,
    dma_rx: DMA1_CH4,
    dma_tx: DMA1_CH5,
) -> SerialWrapper<'static, USART1> {
    let mut uart_config = hal::usart::Config::default();
    uart_config.baudrate = 9600;
    let async_serial = Uart::new(usart, rx, tx, irqs, dma_rx, dma_tx, uart_config).unwrap();

    SerialWrapper::new(async_serial)
}*/
