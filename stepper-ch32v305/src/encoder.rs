#[cfg(any(
    all(feature = "encoder_exti", feature = "encoder_pool"),
    all(feature = "encoder_exti", feature = "encoder_exti2"),
    all(feature = "encoder_exti", feature = "encoder_pool2"),
    all(feature = "encoder_pool", feature = "encoder_exti2"),
    all(feature = "encoder_pool", feature = "encoder_pool2"),
    all(feature = "encoder_exti2", feature = "encoder_pool2"),
))]
compile_error!("Impossible to use two implementations for encoder, please choose one");

#[cfg(not(any(
    feature = "encoder_exti",
    feature = "encoder_exti2",
    feature = "encoder_pool",
    feature = "encoder_pool2",
)))]
compile_error!(
    "In order to use the encoder at least one beetween encoder_exti, encoder_exti2, encoder_pool, encoder_pool2 must be enabled"
);

#[macro_export]
#[collapse_debuginfo(yes)]
///encoder:  PA5 PB10 PB11 PB12
macro_rules! encoder {
    ($p:ident, $spawner:ident, $irq:ident) => {{
        #[cfg(feature = "encoder_exti")]
        let ret = {
            // library inclusion
            use ::ch32_hal::{exti::ExtiInput, gpio::Pull};
            use ::embedcore::{EncoderExti, GetStaticEncoderExti};

            //create encoder
            let e = EncoderExti::<'static>::new(
                ExtiInput::new($p.PA5, $p.EXTI5, Pull::None),
                ExtiInput::new($p.PB10, $p.EXTI10, Pull::None),
            );

            //spawn static
            let stat = e.static_encoder();
            $spawner.must_spawn(::ch32v305::update_encoder_exti(e));
            stat
        };

        #[cfg(feature = "encoder_pool")]
        let ret = {
            use ::ch32_hal::gpio::{Input, Pull};
            use ::embedcore::{EncoderPool, GetStaticEncoderStd};
            let e = EncoderPool::new(
                Input::new($p.PA5, Pull::None),
                Input::new($p.PB10, Pull::None),
                Input::new($p.PB11, Pull::None),
                Input::new($p.PB12, Pull::None),
            );

            let stat = e.static_encoder();
            $spawner.must_spawn(::ch32v305::encoder::update_encoder_pool(e));
            stat
        };
        #[cfg(feature = "encoder_pool2")]
        let ret = {
            use ::ch32_hal::gpio::{Input, Pull};
            use ::embedcore::{EncoderPool2, GetStaticEncoderStd2};
            let e = EncoderPool2::new(
                Input::new($p.PA5, Pull::None),
                Input::new($p.PB10, Pull::None),
            );

            let stat = e.static_encoder();
            $spawner.must_spawn(::ch32v305::encoder::update_encoder_pool2(e));
            stat
        };
        #[cfg(feature = "encoder_exti2")]
        let ret = {
            use ::embedcore::EncoderExti2;
            use ch32_hal::gpio::Pin;
            unsafe {
                EncoderExti2::new(
                    $p.PA5,
                    $p.PB10,
                    $p.EXTI5,
                    $p.EXTI10,
                    $irq,
                    false,
                    $p.PB12.degrade(),
                )
            }
        };
        ret
    }};
}

#[cfg(feature = "encoder_exti")]
#[embassy_executor::task]
/// this is enclosed because it messes globally, so it should be disabled before it expands
pub async fn update_encoder_exti(e: embedcore::EncoderExti<'static>) {
    use embedcore::GetStaticEncoderExti;
    e.update_encoder().await
}

#[cfg(feature = "encoder_pool")]
#[embassy_executor::task]
pub async fn update_encoder_pool(e: embedcore::EncoderPool<ch32_hal::gpio::Input<'static>>) {
    use embedcore::GetStaticEncoderStd;
    e.update_encoder(100_000).await
}

#[cfg(feature = "encoder_pool2")]
#[embassy_executor::task]
pub async fn update_encoder_pool2(e: embedcore::EncoderPool2<ch32_hal::gpio::Input<'static>>) {
    use embedcore::GetStaticEncoderStd2;
    e.update_encoder(80_000).await
}
