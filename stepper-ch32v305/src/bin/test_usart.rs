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
    gpio::{AnyPin, Level, Output, Pin as GpioPin, Speed}, peripherals::USART1, usart::{Config, Uart}
};

use ch32v305::{driver, driver_type, encoder, init, irqs};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::{Duration, Instant, Timer};
use embedcore::{
    common::{
        controllers::pid::{CalibrationMode, PidController},
        motor::{test::{test_basic_movement, test_max_speed}, Motor},
        static_encoder::StaticEncoder,
    }, protocol::{comunication::CommunicationError, cyber::{DeviceIdentifier, Message, MessagesHandler, Response, Slave}, AsyncSerial}, DiscreteDriver, Drv8843Pwm, EncoderTrait, SerialWrapper
};
use serialmessage::{ParseState, SerMsg};


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

#[embassy_executor::task]
async fn message_handler(mut s: Slave<SerialWrapper<'static, USART1>, SerialToMotorHandler>, mut status_pin: Option<Output<'static>>) {
    /*loop {
        let mut buf = ['a' as u8];
        s.com.serial.rx.read(&mut buf).await.unwrap();
        s.com.serial.tx.write(&buf).await.unwrap();
        s.com.serial.tx.write(&buf).await.unwrap();
        s.com.serial.tx.write(&buf).await.unwrap();
        s.com.serial.tx.write(&buf).await.unwrap();
        Timer::after(Duration::from_millis(100)).await;
    }*/
    /*loop {
        if let Some(a) = s.com.try_read_byte().await {
            let buf = [a];
            s.com.serial.tx.write(&buf).await.unwrap();
            s.com.serial.tx.write(&buf).await.unwrap();
            s.com.serial.tx.write(&buf).await.unwrap();
            s.com.serial.tx.write(&buf).await.unwrap();
        } else {
            s.com.serial.tx.write(b"nada").await.unwrap();
        }
    }*/
    /*let mut i = 0;
    let mut id: u8 = 0;
    loop {
        if let Some(a) = s.com.try_read_byte().await {
            if a == 126 {
                if i != 0 {
                    status_pin.as_mut().map(|p| (p.toggle(),));
                }
                i = 0;
            }
            if i == 1 {
                id = a;
            }
            if i == 7 {
                let resp = Response::Iam(DeviceIdentifier { name: *b"ciao123456", version: 1 });
                let mut buf = [0; 32];
                let buf2 = postcard::to_slice(&resp, &mut buf).unwrap();
                let buf3 = SerMsg::create_msg_arr(buf2, id).unwrap();
                s.com.serial.tx.write(&buf3.0[..buf3.1]).await.unwrap();
            }
        } else {
            //status_pin.as_mut().map(|p| (p.toggle(),));
        }
        i += 1;
    }*/
    /*loop {
        match s.com.try_read::<Message>().await {
            Ok((id, _m)) => {
                let resp = Response::Iam(DeviceIdentifier { name: *b"ciao123456", version: 1 });
                let mut buf = [0; 32];
                let buf2 = postcard::to_slice(&resp, &mut buf).unwrap();
                let buf3 = SerMsg::create_msg_arr(buf2, id).unwrap();
                s.com.serial.tx.write(&buf3.0[..buf3.1]).await.unwrap();
            },
            Err(CommunicationError::CantRead) => {
                status_pin.as_mut().map(|p| (p.toggle(),));
            },
            Err(CommunicationError::InvalidMsg) => {
                //status_pin.as_mut().map(|p| (p.toggle(),));
            }
            _ => {
                status_pin.as_mut().map(|p| (p.toggle(),));
            }
        }
    }*/
    /*loop {
        if let Ok(a) = s.com.try_read::<Message>().await {
            s.com.send( Response::Iam(DeviceIdentifier { name: *b"ciao123456", version: 1 }), a.0).await;
        } else {
            status_pin.as_mut().map(|p| (p.toggle(),));
            if let Some(pin) = &mut status_pin { pin.toggle() }
        }
    }*/
    s.run().await
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

    /*let (mut tx, mut rx) = serial.split();
    loop {
        let mut buf = [0u8];
        rx.read(&mut buf).await.unwrap();
        tx.write(&buf).await.unwrap();
        tx.write(&buf).await.unwrap();
        tx.write(&buf).await.unwrap();
        tx.write(&buf).await.unwrap();
        Timer::after(Duration::from_millis(100)).await;
    }*/

    /*let motor = Motor::new(e, d, false);/sys/class/tty
    let mut pid = PidController::new(motor, 1.8, 1.8);

    pid.calibration(2000, CalibrationMode::NoOvershoot).await;*/

    //spawner.spawn(blink(p.PA4.degrade(), 1000)).unwrap();
    let mh = SerialToMotorHandler::new(p.PA4.degrade());

    // spawn message handler thread
    let serial_wrapper = SerialWrapper::new(serial, None);
    let s: Slave<SerialWrapper<'static, USART1>, _> = Slave::new(serial_wrapper, 10000, *b"ciao      ", mh);
    spawner.must_spawn(message_handler(s, None));

    //spawner.must_spawn(update_motor(pid));
    //let mut out = Output::new(p.PA4, Level::High, Speed::High);
    let e = encoder!(p, spawner, IrqsExti);
    let d = driver!(p, spawner);
    let mut motor = Motor::new(e, d, true);



    let mut pid = PidController::new(motor, 2.0, 2.0);
    //pid.motor.align(1.0, 1.0).await;
    let _ = pid.calibration(2000, CalibrationMode::NoOvershoot).await;
    //info!("Motor initialized");

    loop {
        defmt_or_log::info!("{}", pid.motor.read());

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
        }
    }
    loop {
        //out.toggle();
        //let mut c = [55u8];
        //let _= serial.bloPA4cking_read(&mut c);

        //let c = tx.write(&c).await;
        //info!("write: {}", c);
        // let c = tx.blocking_flush();
        // info!("flushed {}", c);
        /*for i in 0..80{bloPA4cking_read
            pid.try_from::<DE>().unwrap().set_phase(i%80, 1.0);
            yield_now().await;

        }*/
        //pid.update(1.0.into());

        motor.set_phase(10, 0.5);
        test_max_speed(&mut motor, true).await;
        Timer::after_secs(1).await;
        test_max_speed(&mut motor, false).await;
        Timer::after_secs(1).await;
    }
    //x.write(buffer)
    //SerialWrapper::new(tx);*/
}
