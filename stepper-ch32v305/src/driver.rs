#[cfg(all(feature = "driver_dac", feature = "driver_pwm"))]
compile_error!("Impossible to use two implementations for driver, please choose one");

#[cfg(not(any(feature = "driver_dac", feature = "driver_pwm")))]
compile_error!(
    "In order to use the driver at least one beetween driver_dac or driver_pwm must be enabled"
);

#[macro_export]
#[collapse_debuginfo(yes)]
/// driver pwm(A, B): TIM3 PC7(2) PC8(3)
macro_rules! driver {
    ($p:ident, $spawner:ident) => {{
        #[cfg(feature = "driver_pwm")]
        let driver = {
            use ch32_hal::gpio::{Input, Level, Output, Pull, Speed};
            use ch32_hal::peripherals::{TIM3, TIM8};
            use ch32_hal::prelude::Hertz;
            use ch32_hal::timer::{
                Channel,
                low_level::CountingMode,
                simple_pwm::{PwmPin, SimplePwm},
            };
            use embedcore::Drv8843Pwm;

            //driver reset fault decay: PB13 PB14 PC6
            let reset = Output::new($p.PB13, Level::Low, Speed::Low);
            let nfault = Input::new($p.PB14, Pull::Up);
            //let decay = Output::new($p.PC6, Level::Low, Speed::Low);
            //driver_dir(A, B): PB6 PB7
            let BIN = (
                Output::new($p.PB6, Level::Low, Speed::High),
                Output::new($p.PC11, Level::High, Speed::High),
            );
            let AIN = (
                Output::new($p.PB7, Level::Low, Speed::High),
                Output::new($p.PC12, Level::High, Speed::High),
            );

            /// driver pwm(A, B): PA7=> A
            /// driver pwm(A, B): PC8=> B
            let mut pwma = SimplePwm::new(
                $p.TIM3,
                None,
                Some(PwmPin::new_ch2::<0>($p.PA7)),
                None,
                None,
                Hertz::khz(100),
                CountingMode::EdgeAlignedUp,
            );
            let mut pwmb = SimplePwm::new(
                $p.TIM8,
                None,
                None,
                Some(PwmPin::new_ch3::<0>($p.PC8)),
                None,
                Hertz::khz(100),
                CountingMode::EdgeAlignedUp,
            );
            pwma.enable(Channel::Ch2);
            pwmb.enable(Channel::Ch3);
            Drv8843Pwm::<'static, TIM3, TIM8, 20>::new(
                reset,
                nfault,
                $p.PA6.into(),
                AIN,
                BIN,
                pwma,
                pwmb,
                Channel::Ch2,
                Channel::Ch3,
            )
        };
        driver
    }};
}
#[macro_export]
#[collapse_debuginfo(yes)]
macro_rules! driver_type {
    () => {
        Drv8843Pwm::<'static, ch32_hal::peripherals::TIM3, ch32_hal::peripherals::TIM8, 20>
    };
}
