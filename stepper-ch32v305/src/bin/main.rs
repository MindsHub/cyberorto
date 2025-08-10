#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(naked_functions)]
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
                    Timer::after(Duration::from_micros(500)).await;
                }
                SHARED.lock(|x| {
                    x.borrow_mut().cmd = CMD::Waiting;
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
    let e = encoder!(p, spawner, IrqsExti);
    //let mut e = encoder(p.PD1, p.PD2, p.PD3, p.PD4, &spawner);
    /*let d = ch32v305::driver!(p, spawner);*/
    /*let mut serial: SerialWrapper<'_, USART1> =
        serial(p.USART1, p.PA8, p.PB15, IrqsUsart, p.DMA1_CH4, p.DMA1_CH5);
*/
let mut serial = Uart::new(p.USART1, p.PA8, p.PB15, IrqsUsart, p.DMA1_CH4, p.DMA1_CH5, Config::default()).unwrap();
    /*let motor = Motor::new(e, d, false);
    let mut pid = PidController::new(motor, 1.8, 1.8);

    pid.calibration(2000, CalibrationMode::NoOvershoot).await;*/

   /* let mh = SerialToMotorHandler::new(p.PA4.degrade());

    // spawn message handler thread
    let s: Slave<SerialWrapper<'static, USART1>, _> = Slave::new(serial, 100, *b"ciao      ", mh);
    spawner.must_spawn(message_handler(s));*/
    //spawner.must_spawn(update_motor(pid));
    let mut out = Output::new(p.PA4, Level::High, Speed::High);
    loop {
        out.toggle();
        let mut c = [55u8];
        //let _= serial.blocking_read(&mut c);

        serial.blocking_write(&c);
        serial.blocking_flush();
        /*for i in 0..80{
            pid.try_from::<DE>().unwrap().set_phase(i%80, 1.0);
            yield_now().await;

        }*/
        //pid.update(1.0.into());
        Timer::after(Duration::from_millis(100)).await;
    }
    //x.write(buffer)
    //SerialWrapper::new(tx);*/
}
