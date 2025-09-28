#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case, unsafe_op_in_unsafe_fn, unused_imports, unused_mut)]
/*!
 * Monolitic code test, first version of the implementation of the driver. if necessary, it could be best to modulirize a little more */

use core::{cell::RefCell, cmp::min};

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
                motor.reset(&finecorsa, 1.0).await;
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

    // setup encoder and decoder
    let e = encoder!(p, spawner, IrqsExti);
    let d = driver!(p, spawner);
    let mut motor = Motor::new(e, d, /* rotation = */ true);

    // choose PID parameters
    let mut pid = PidController::new(motor, 1.0, 1.0);
    pid.pid.p(0.005, 2.0);
    pid.pid.i(0.0005, 1.0);
    pid.pid.d(-0.0005, 0.5);
    //0.046299618, i=0.000670455, d=0.00044697
    //info!("Motor initialized");

    let finecorsa = Input::new(p.PC2, ch32_hal::gpio::Pull::Up);
    // wait a little time, otherwise the finecorsa pin will read low although it's high
    Timer::after_micros(10).await;

    // reset when turning on the motor (may be removed)
    AsseZ::reset(&mut pid, &finecorsa, 1.0).await;

    // start motor task which will keep the motor running
    spawner.must_spawn(update_motor(pid, finecorsa));
    loop {
        Timer::after_secs(100).await;
    }
}

trait AsseZ {
    /// How many microseconds to remain in each phase before moving on to the next.
    const RESET_PHASE_US: u64;
    /// How many milliseconds to remain still to wait for the position to settle during alignement.
    const RESET_ALIGN_MS: u64;
    /// How much to go down before going back up if the finecorsa was already
    /// active when starting the reset procedure. It's better if this is a multiple of
    /// the number of microsteps (80), so the motor doesn't jump ahead suddenly, but it's
    /// not so important.
    const RESET_SAFE_DOWN_MICROSTEPS: u32;
    /// How much further up can the axis move after the finecorsa has been activated?
    /// MUST BE A MULTIPLE OF THE NUMBER OF MICROSTEPS (80)
    const RESET_OFFSET_MICROSTEPS: u32;
    /// The number of phases for a full turn of the motor.
    const PHASE_COUNT: u8;

    // Some static asserts
    const _PHASE_COUNT_ASSERT: () = assert!(Self::PHASE_COUNT == 80);
    const _RESET_OFFSET_ASSERT: () = assert!(Self::RESET_OFFSET_MICROSTEPS % (Self::PHASE_COUNT as u32) == 0);

    /// Little helper to do i%PHASE_COUNT.
    fn to_phase(i: i32) -> u8 {
        i.rem_euclid(Self::PHASE_COUNT as i32) as u8
    }

    /// Reset the Z axis using the finecorsa.
    async fn reset(&mut self, finecorsa: &Input<'static>, current: f32) -> ResetResult;
}

#[must_use]
enum ResetResult {
    Ok,
    ReachingFinecorsaTimedOut,
}

impl AsseZ for PidController<StaticEncoder, driver_type!()> {
    const RESET_PHASE_US: u64 = 100;
    const RESET_ALIGN_MS: u64 = 500;
    const RESET_SAFE_DOWN_MICROSTEPS: u32 = 10_000;
    const RESET_OFFSET_MICROSTEPS: u32 = 2_560;
    const PHASE_COUNT: u8 = (<driver_type!()>::MICROSTEP * 4) as u8;

    async fn reset(&mut self, finecorsa: &Input<'static>, current: f32) -> ResetResult {
        // Note: in this function we don't use PID to move the motor, nor do we use
        // feedback for keeping track of the motor position. We just set maximum current
        // and cycle through the microsteps one at a time, as if this was a normal stepper
        // without feedback. This means that the motor will be able to move in the right
        // direction even if completely misaligned.

        // represents the next phase that will be set on the motor (mod PHASE_COUNT)
        let mut i = 0i32;
        let mut result = ResetResult::Ok;

        // - if the finecorsa is already active move down a bit
        //   to avoid resetting at the wrong position
        // - otherwise don't do anything, to avoid hitting obstacles below
        //   (e.g. imagine the end effector is already touching the ground)
        if finecorsa.is_low() {
            for _ in 0..Self::RESET_SAFE_DOWN_MICROSTEPS {
                self.motor.set_phase(Self::to_phase(i), current);
                i += 1; // go down, so add
                Timer::after_micros(Self::RESET_PHASE_US).await;
            }
        }

        // move up until the finecorsa is activated
        let start = Instant::now();
        while finecorsa.is_high() {
            if start.elapsed().as_secs() >= 10 {
                result = ResetResult::ReachingFinecorsaTimedOut;
                break;
            }
            self.motor.set_phase(Self::to_phase(i), current);
            i -= 1; // go up, so subtract
            Timer::after_micros(Self::RESET_PHASE_US).await;
        }

        // we continue moving up for Self::RESET_OFFSET_MICROSTEPS,
        // and then keep going a bit more until we reach a phase of 0
        i = Self::to_phase(i) as i32;
        loop {
            self.motor.set_phase(Self::to_phase(i), current);
            if i < -(Self::RESET_OFFSET_MICROSTEPS as i32) && Self::to_phase(i) == 0 {
                break;
            }
            i -= 1; // go up, so subtract
            Timer::after_micros(Self::RESET_PHASE_US).await;
        }

        // wait some time for the motor to settle down, so we can be sure that the motor has
        // physically reached the position corresponding to phase 0
        Timer::after_millis(Self::RESET_ALIGN_MS).await;

        // Set the motor's 0 position to the current position. Note that this requires the
        // physical phase to be aligned to 0, since `self.motor.shift = -current_position`
        // will make it so that if we called `self.motor.read()` we would get 0, and that's
        // also what's used in `self.motor.set_current()`.
        let current_position = self.motor.encoder.read();
        self.motor.shift = -current_position;

        // make sure the objective corresponds with the current position (i.e. 0)
        self.set_objective(0);
        // turn off motor
        self.motor.set_phase(0, 0.0);

        result
    }
}
