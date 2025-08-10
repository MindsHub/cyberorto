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
    gpio::{AnyPin, Level, Output, Speed},
    peripherals::USART1,
};

use ch32v305::{encoder, driver, init, irqs, serial};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::{Duration, Timer};
use embedcore::{
    common::{
        controllers::pid::{CalibrationMode, PidController},
        motor::Motor,
    }, protocol::{cyber::{MessagesHandler, Response, Slave}}, EncoderTrait, SerialWrapper
};
use defmt_or_log::info;

#[derive(PartialEq, Eq, Clone, defmt::Format)]
pub enum Cmd {
    Reset,
    MoveTo(i32),
    Waiting,
    Error([u8; 10]),
}
struct Shared {
    pub cmd: Cmd,
}

static SHARED: Mutex<CriticalSectionRawMutex, RefCell<Shared>> =
    Mutex::new(RefCell::new(Shared { cmd: Cmd::Waiting }));

irqs!();

struct SerialToMotorHandler {
    status_pin: Option<Output<'static>>,
}

impl SerialToMotorHandler {
    fn new(status_pin: Option<AnyPin>) -> Self {
        let status_pin = status_pin.map(|p| Output::new(p, Level::High, Speed::High));
        Self { status_pin }
    }
}

// implementing MessageHandler, if some message should not have any response, it will return None
impl MessagesHandler for SerialToMotorHandler {
    async fn set_led(&mut self, state: bool) -> Option<Response> {
        info!("Set led {:?}", state);
        if state {
            if let Some(status_pin) = &mut self.status_pin {
                status_pin.set_high();
            }
        } else {
            if let Some(status_pin) = &mut self.status_pin {
                status_pin.set_low();
            }
        }
        Some(Response::Done)
    }
    async fn move_motor(&mut self, x: f32) -> Option<Response> {
        SHARED.lock(|shared| {
            shared.borrow_mut().cmd = Cmd::MoveTo(x as i32);
        });
        Some(Response::Wait { ms: 1000 })
    }
    async fn reset_motor(&mut self) -> Option<Response> {
        SHARED.lock(|shared| {
            shared.borrow_mut().cmd = Cmd::Reset;
        });
        Some(Response::Wait { ms: 1000 })
    }
    async fn poll(&mut self) -> Option<Response> {
        SHARED.lock(|shared| match shared.borrow().cmd {
            Cmd::Waiting => Some(Response::Done),
            Cmd::Error(c) => Some(Response::Debug(c)),
            _ => Some(Response::Wait { ms: 1000 }),
        })
    }
}

#[embassy_executor::task]
async fn message_handler(mut s: Slave<SerialWrapper<'static, USART1>, SerialToMotorHandler>) {
    s.run().await
}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let p = init();
    Timer::after_millis(300).await;

    // spawn message handler thread
    let serial_wrapper = serial(p.USART1, p.PA8, p.PB15, IrqsUsart, p.DMA1_CH4, p.DMA1_CH5);
    let mh = SerialToMotorHandler::new(None);//p.PA4.degrade());
    let s: Slave<SerialWrapper<'static, USART1>, _> = Slave::new(serial_wrapper, 10000, *b"p         ", mh);
    spawner.must_spawn(message_handler(s));

    // setup motor
    let e = encoder!(p, spawner, IrqsExti);
    let d = driver!(p, spawner);
    let motor = Motor::new(e, d, true);
    let mut pid = PidController::new(motor, 2.0, 2.0);
    info!("Motor initialized");

    // motor handling loop
    loop {
        let cur = SHARED.lock(|x| x.borrow().cmd.clone());
        match cur {
            Cmd::MoveTo(x) => {
                pid.set_objective(x);
                while (pid.motor.read() - x).abs() > 10 {
                    pid.update().await;
                    embassy_futures::yield_now().await;
                }
                SHARED.lock(|x| {
                    x.borrow_mut().cmd = Cmd::Waiting;
                });

            }
            Cmd::Reset => {
                let cur_pos = pid.motor.read();
                pid
                    .calibration(cur_pos + 2000, CalibrationMode::NoOvershoot)
                    .await;
                pid.set_objective(cur_pos);
                while (pid.motor.read() - cur_pos).abs() > 10 {
                    pid.update().await;
                    Timer::after(Duration::from_micros(500)).await;
                }
                SHARED.lock(|x| {
                    x.borrow_mut().cmd = Cmd::Waiting;
                });
            }
            _ => {}
        }
        Timer::after_millis(10).await;
    }
}