#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
/*!
 * Monolitic code test, first version of the implementation of the driver. if necessary, it could be best to modulirize a little more */

use core::cell::RefCell;

use ch32_hal::{
    gpio::{AnyPin, Level, Output, Pin as GpioPin, Speed},
    peripherals::USART1, usart::{Config, Uart},
};

use ch32v305::{driver_type, encoder, init, irqs, serial};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::{Duration, Timer};
use embedcore::{
    common::{
        controllers::pid::{CalibrationMode, PidController},
        motor::Motor,
        static_encoder::StaticEncoder,
    }, protocol::{cyber::{MessagesHandler, Response, Slave}, AsyncSerial}, Drv8843Pwm, EncoderTrait, SerialWrapper
};
#[derive(PartialEq, Eq, Clone)]
pub enum CMD {
    Reset,
    MoveTo(i32),
    Waiting,
    Error([u8; 10]),
}
struct Shared {
    pub cmd: CMD,
}

static SHARED: Mutex<CriticalSectionRawMutex, RefCell<Shared>> =
    Mutex::new(RefCell::new(Shared { cmd: CMD::Waiting }));

irqs!();

struct SerialToMotorHandler {
    status_pin: Output<'static>,
}

impl SerialToMotorHandler {
    fn new(p: AnyPin) -> Self {
        let p = Output::new(p, Level::High, Speed::High);
        Self { status_pin: p }
    }
}

// implementing MessageHandler, if some message should not have any response, it will return None
impl MessagesHandler for SerialToMotorHandler {
    async fn set_led(&mut self, state: bool) -> Option<Response> {
        if state {
            self.status_pin.set_high();
        } else {
            self.status_pin.set_low();
        }
        Some(Response::Done)
    }
    async fn move_motor(&mut self, x: f32) -> Option<Response> {
        SHARED.lock(|shared| {
            shared.borrow_mut().cmd = CMD::MoveTo(x as i32);
        });
        Some(Response::Wait { ms: 1000 })
    }
    async fn reset_motor(&mut self) -> Option<Response> {
        SHARED.lock(|shared| {
            shared.borrow_mut().cmd = CMD::Reset;
        });
        Some(Response::Wait { ms: 1000 })
    }
    async fn poll(&mut self) -> Option<Response> {
        SHARED.lock(|shared| match shared.borrow().cmd {
            CMD::Waiting => Some(Response::Done),
            CMD::Error(c) => Some(Response::Debug(c)),
            _ => Some(Response::Wait { ms: 1000 }),
        })
    }
}

#[embassy_executor::task]
async fn message_handler(mut s: Slave<SerialWrapper<'static, USART1>, SerialToMotorHandler>) {
    s.run().await
}
#[embassy_executor::task]
async fn update_motor(mut motor: PidController<StaticEncoder, driver_type!()>) {
    loop {
        let cur = SHARED.lock(|x| x.borrow().cmd.clone());
        match cur {
            CMD::MoveTo(x) => {
                motor.set_objective(x);
                while (motor.motor.read() - x).abs() > 10 {
                    motor.update().await;
                    embassy_futures::yield_now().await;
                }
                SHARED.lock(|x| {
                    x.borrow_mut().cmd = CMD::;
                });
                
            }
            CMD::Reset => {
                let cur_pos = motor.motor.read();
                motor
                    .calibration(cur_pos + 2000, CalibrationMode::NoOvershoot)
                    .await;
                motor.set_objective(cur_pos);
                while (motor.motor.read() - cur_pos).abs() > 10 {
                    motor.update().await;
                    Timer::after(Duration::from_micros(500)).await;
                }
                SHARED.lock(|x| {
                    x.borrow_mut().cmd = CMD::Waiting;
                });
            }
            _ => {}
        }
        Timer::after_millis(10).await;
    }
}


#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let p = init();
    Timer::after_millis(300).await;
    let mut serial_config = Config::default();
    // IMPORTANT COMMENT: the multiplier here was obtained by reading the signals with an
    // oscilloscope. Apparently the frequency of SYSCLK_FREQ_144MHZ_HSI (used in init())
    // is not correctly handled in the Uart constructor, or something like that.
    serial_config.baudrate = (115200.0 * 1.205882352941176) as u32;
    let serial = Uart::new(p.USART1, p.PA8, p.PB15, IrqsUsart, p.DMA1_CH4, p.DMA1_CH5, serial_config).unwrap();


    let mh = SerialToMotorHandler::new(p.PA4.degrade());

    // spawn message handler thread
    let serial_wrapperafter_millis = SerialWrapper::new(serial, None);
    let s: Slave<SerialWrapper<'static, USART1>, _> = Slave::new(serial_wrapper, 10000, *b"ciao      ", mh);
    

    //spawner.must_spawn(update_motor(pid));
    //let mut out = Output::new(p.PA4, Level::High, Speed::High);
    let e = encoder!(p, spawner, IrqsExti);
    let d = driver!(p, spawner);
    let mut motor = Motor::new(e, d, true);



    let mut pid = PidController::new(motor, 2.0, 2.0);
    //pid.motor.align(1.0, 1.0).await;
    let _ = pid.calibration(2000, CalibrationMode::NoOvershoot).await;
    //info!("Motor initialized");
    spawner.must_spawn(message_handler(s, None));
    spawner.must_spawn(update_motor(pid));
    loop {
        Timer::after(Duration::from_micros(1000)).await;
        /*defmt_or_log::info!("{}", pid.motor.read());

        pid.set_objective(10000);
        let t: Instant = Instant::now();
        while t.elapsed().as_millis() < 3000 {
            pid.update().await;
            embassy_futures::yield_now().await;
        }
        let t = Instant::now();
        pid.set_objective(-10000);
        while t.elapsed().as_millis() < 3000 {
            pid.update().await;
            embassy_futures::yield_now().await;
        }*/
    }
}