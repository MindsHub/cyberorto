#[macro_export]
#[collapse_debuginfo(yes)]
macro_rules! irqs {
    () => {
        pub mod irqs_mod{
            use ch32_hal::{bind_interrupts, peripherals, usart};
            use embedcore::RedirectExtiToEncoder;


            bind_interrupts!(pub struct IrqsUsart {
                USART1 => usart::InterruptHandler<peripherals::USART1>;
            });

            #[cfg(feature = "encoder_exti2")]

            bind_interrupts!(pub struct IrqsExti {
                EXTI9_5 => RedirectExtiToEncoder<ch32_hal::peripherals::EXTI5>;
                EXTI15_10 => RedirectExtiToEncoder<ch32_hal::peripherals::EXTI10>;
            });
            #[cfg(not(feature = "encoder_exti2"))]
            pub struct Exti;
        }
        pub use irqs_mod::*;

    };
}
