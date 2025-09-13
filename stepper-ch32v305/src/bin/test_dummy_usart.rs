#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![feature(int_format_into)]

use ch32_hal::usart::Uart;
use ch32_hal as hal;
use ch32v305::{init, irqs};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use hal::gpio::{AnyPin, Level, Output, Pin};
use core::fmt::NumBuffer;

//bind_interrupts!(struct Irqs {
//    USART4 => usart::InterruptHandler<peripherals::USART4>;
//});

//#[no_mangle]
//unsafe extern "C" fn DMA1_CHANNEL1() {
//    println!("in irq");
//}

/*
pub const SYSCLK_FREQ_144MHZ_HSI: Config = {
        Config {
            hse: None,
            sys: Sysclk::PLL,
            pll_src: PllSource::HSI,
            pll: Some(Pll {
                prediv: PllPreDiv::DIV1,
                mul: PllMul::MUL18,
            }),
            pllx: None,
            ahb_pre: AHBPrescaler::DIV1,
            apb1_pre: APBPrescaler::DIV1,
            apb2_pre: APBPrescaler::DIV1,
            ls: super::LsConfig::default_lsi(),
            hspll_src: HsPllSource::HSI,
            hspll: Some(HsPll {
                pre: HsPllPrescaler::DIV2,
            }),
        }
    };

    Self {
            // hsi: true,
            hse: None,
            sys: Sysclk::HSI,
            pll_src: PllSource::HSI,
            pll: None,
            pllx: None,
            ahb_pre: AHBPrescaler::DIV1,
            apb1_pre: APBPrescaler::DIV1,
            apb2_pre: APBPrescaler::DIV1,
            ls: super::LsConfig::default(),
            hspll_src: HsPllSource::HSE,
            hspll: None,
        }
pub const MISCIOTTO: Config = {
    Config {
        rcc: SYSCLK_FREQ_144MHZ_HSI,
        dma_interrupt_priority: interrupt::Priority::P0,
    }
};*/
#[embassy_executor::task(pool_size = 3)]
async fn blink(pin: AnyPin, interval_ms: u64) {
    let mut led = Output::new(pin, Level::Low, Default::default());

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(interval_ms)).await;
        led.set_low();
        Timer::after(Duration::from_millis(interval_ms)).await;
    }
}
irqs!();

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let p = init();
    Timer::after_millis(1000).await;
    //defmt_or_log::error!("ciao\n");
    /*hal::debug::SDIPrint::enable();
    let mut config = hal::Config::default();
    config.rcc = hal::rcc::Config::SYSCLK_FREQ_144MHZ_HSE;
    config.dma_interrupt_priority = interrupt::Priority::P0;
    config = Default::default();*/
    
    //let p = hal::init(config);

    // Connector pinout:
    // GND, VCC, PC17, PC16
    // GND, VCC, SDA, SCL (I2C1) - not working
    // GND, VCC, TX, RX (USART4, remap=5)
    // GND, VCC, RX, TX (USART4, remap=2)

    let uart_config = hal::usart::Config::default();
    //uart_config.baudrate = 9600;
    /*T::enable_and_reset();

        let rb = T::regs();
        rb.ctlr3().modify(|w| w.set_ctse(cts.is_some()));
        configure(&rb, &config, T::frequency(), true, false)?;

    //let (mut tx, mut rx) = Uart::new(p.USART1, p.PA8, p.PB15
        // create state once!
        let _s = T::state();*/
    let (mut tx, mut rx) = Uart::new(p.USART1, p.PA8, p.PB15, IrqsUsart, p.DMA1_CH4, p.DMA1_CH5, uart_config).unwrap().split();
    //let mut tx = UartTx::new(p.USART1, p.PB15, p.DMA1_CH4, uart_config).unwrap();
    //let mut rx = UartRx::new(p.USART1, IrqsUsart, p.PA8, p.DMA1_CH5, uart_config).unwrap();
    // GPIO
    // let mut led = Output::new(p.PB12, Level::High, Default::default());
    spawner.spawn(blink(p.PA4.degrade(), 1000)).unwrap();

    //let buf = b"Hello World\r\n";
    let mut i: i32 = 0;
    let mut ciao = [0; 1];
    let mut num_buffer = NumBuffer::new();
    loop {
        //defmt_or_log::error!("ciao: {:?}", ciao);
        //tx.write(buf).await.unwrap();
        i += 1;
        tx.write(i.format_into(&mut num_buffer).as_bytes()).await.unwrap();
        tx.write(b" ").await.unwrap();
        tx.write((ciao[0] as i32).format_into(&mut num_buffer).as_bytes()).await.unwrap();
        tx.write(b"\r\n").await.unwrap();
        rx.read(&mut ciao).await.unwrap();
        //tx.blocking_flush();
        Timer::after_millis(100).await;
        // defmt_or_log::info!("ciao");
        // led.toggle();
    }
}
