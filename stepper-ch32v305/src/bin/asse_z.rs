#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case, unsafe_op_in_unsafe_fn, unused_imports, unused_mut)]
/*!
 * Monolitic code test, first version of the implementation of the driver. if necessary, it could be best to modulirize a little more */

use core::cell::RefCell;

use ch32_hal::{
    gpio::{AnyPin, Input, Level, Output, Pin as GpioPin, Speed},
    peripherals::USART1,
    usart::{Config, Uart},
};

use ch32v305::{driver, driver_type, encoder, init, irqs};
use defmt_or_log::info;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::{Duration, Instant, Timer};
use embedcore::{
    DiscreteDriver, Drv8843Pwm, EncoderTrait, SerialWrapper,
    common::{
        controllers::pid::{CalibrationMode, PidController},
        motor::{
            Motor,
            test::{test_basic_movement, test_max_speed},
        },
        static_encoder::StaticEncoder,
    },
    protocol::{
        AsyncSerial,
        communication::CommunicationError,
        cyber::{DeviceIdentifier, Message, MessagesHandler, MotorState, Response, Slave},
    },
};
use qingke::riscv::register::satp::set;
use serialmessage::{ParseState, SerMsg};

#[derive(PartialEq, Eq, Clone)]
pub enum Cmd {
    Reset,
    MoveTo(i32),
    Idle,
    Error([u8; 10]),
}
struct Shared {
    pub cmd: Cmd,
    //pub reset: Bool,
}

static SHARED: Mutex<CriticalSectionRawMutex, RefCell<Shared>> =
    Mutex::new(RefCell::new(Shared { cmd: Cmd::Idle }));

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

// implementing MessageHandler, if some message should not have any response, it will return Unsupported
impl MessagesHandler for SerialToMotorHandler {
    async fn get_motor_state(&mut self) -> Response {
        SHARED.lock(|shared| {
            let motor_pos = 0f32; // TODO
            let (is_idle, error) = match &shared.borrow().cmd {
                Cmd::Idle => (true, None),
                Cmd::Error(e) => (true, Some(*e)),
                _ => (false, None),
            };
            Response::MotorState(MotorState {
                motor_pos,
                is_idle,
                error,
            })
        })
    }
    async fn reset_motor(&mut self) -> Response {
        SHARED.lock(|shared| {
            shared.borrow_mut().cmd = Cmd::Reset;
        });
        Response::Ok
    }
    async fn move_motor(&mut self, x: f32) -> Response {
        SHARED.lock(|shared| {
            shared.borrow_mut().cmd = Cmd::MoveTo(x as i32);
        });
        Response::Ok
    }
    async fn set_led(&mut self, state: bool) -> Response {
        if state {
            self.status_pin.set_high();
        } else {
            self.status_pin.set_low();
        }
        Response::Ok
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
async fn message_handler(mut s: Slave<SerialWrapper<'static, USART1>, SerialToMotorHandler>) {
    s.run().await
}

#[embassy_executor::task]
async fn update_motor(
    mut motor: PidController<StaticEncoder, driver_type!()>,
    finecorsa: Input<'static>,
) {
    let mut instant = Instant::now();
    loop {
        let cur = SHARED.lock(|x| x.borrow().cmd.clone());
        match cur {
            Cmd::MoveTo(x) => {
                motor.set_objective(x);
                while (motor.motor.read() - x).abs() > 5 {
                    motor.update().await;
                    Timer::after(Duration::from_micros(500)).await;
                    if instant.elapsed().as_millis() > 1000 {
                        let cur = motor.motor.read();
                        let setpoint = motor.pid.setpoint as i32;
                        info!("Motor position: {} {} {}", cur, setpoint, cur - setpoint);
                        instant = Instant::now();
                    }
                }
                motor.update().await;
                SHARED.lock(|x| {
                    x.borrow_mut().cmd = Cmd::Idle;
                });
            }
            Cmd::Reset => {
                motor.reset(&finecorsa).await;
                SHARED.lock(|x| {
                    x.borrow_mut().cmd = Cmd::Idle;
                });
            }
            _ => {
                motor.update().await;
            }
        }
        Timer::after_micros(100).await;
    }
}
#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let p = init();
    Timer::after_secs(5).await;
    let mut serial_config = Config::default();
    // IMPORTANT COMMENT: the multiplier here was obtained by reading the signals with an
    // oscilloscope. Apparently the frequency of SYSCLK_FREQ_144MHZ_HSI (used in init())
    // is not correctly handled in the Uart constructor, or something like that.
    //serial_config.baudrate = (115200.0 * 1.205882352941176) as u32;
    let serial = Uart::new(
        p.USART1,
        p.PA8,
        p.PB15,
        IrqsUsart,
        p.DMA1_CH4,
        p.DMA1_CH5,
        serial_config,
    )
    .unwrap();

    let mh = SerialToMotorHandler::new(p.PA4.degrade());

    // spawn message handler thread
    let serial_wrapper = SerialWrapper::new(serial, None);
    let s: Slave<SerialWrapper<'static, USART1>, _> =
        Slave::new(serial_wrapper, *b"z         ", mh);
    spawner.must_spawn(message_handler(s));

    let e = encoder!(p, spawner, IrqsExti);
    let d = driver!(p, spawner);
    let mut motor = Motor::new(e, d, true);

    let mut pid = PidController::new(motor, 1.0, 1.0);
    pid.pid.p(0.005, 2.0);
    pid.pid.i(0.0005, 1.0);
    pid.pid.d(-0.0005, 0.5);
    //0.046299618, i=0.000670455, d=0.00044697
    //info!("Motor initialized");

    let finecorsa = Input::new(p.PC2, ch32_hal::gpio::Pull::Up);
    Timer::after_micros(10).await;
    AsseZ::reset(&mut pid, &finecorsa).await;

    // pid.set_objective(10_000);
    // let start = Instant::now();
    // while start.elapsed().as_secs() < 5 {
    //     pid.update().await;
    //     Timer::after_micros(500).await;
    // }
    // pid.set_objective(0);
    // let start = Instant::now();
    // while start.elapsed().as_secs() < 5 {
    //     pid.update().await;
    //     Timer::after_micros(500).await;
    // }

    spawner.must_spawn(update_motor(pid, finecorsa));
    loop {
        //pid.update().await;
        Timer::after_secs(100).await;
    }
}

trait AsseZ {
    async fn reset(&mut self, finecorsa: &Input<'static>, current: f32);
}

impl AsseZ for PidController<StaticEncoder, driver_type!()> {
    async fn reset(&mut self, finecorsa: &Input<'static>, current: f32) {
        // move down a bit if the finecorsa is already active,
        // to avoid resetting at the wrong position
        if finecorsa.is_low() {
            for i in 0..10_000 {
                self.motor.set_phase((i % 80) as u8, current);
                Timer::after_micros(100).await;
            }
        }

        // move up until the finecorsa is activated
        let start = Instant::now();
        let mut i = 0;
        while finecorsa.is_high() && start.elapsed().as_secs() < 10 {
            self.motor.set_phase(80 - (i % 80) as u8, current);
            i += 1;
            Timer::after_micros(100).await;
        }

        let current_position = self.motor.read();
        // it must be a multiple of 80
        self.motor.shift(-current_position+2_560);
        let start = Instant::now();
        let mut i = 0;
        while self.motor.read() > 0 && start.elapsed().as_secs() < 1 {
            self.motor.set_phase(80 - (i % 80) as u8, current);
            i += 1;
            Timer::after_micros(100).await;
        }
        self.motor.align(current, 0.3).await;
    }
}
