#[macro_export]
macro_rules! get_name {
    ($name:tt $($rep:tt)+) => {
        get_name!($($rep)+)
    };
    ($name:tt) =>{
        $name
    }
}

#[macro_export]
macro_rules! impl_discrete_driver {
    ( $name:ident $(< $( $($lt:ident)+  $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)?, $access:ident, $step:expr) => {
        impl $(< $( $($lt )+  $( : $clt $(+ $dlt )* )? ),+ >)?
            DiscreteDriver
        for $name
            $(< $( get_name!($($lt)+) ),+ >)?
        {
            const MICROSTEP: usize=$step;
            fn set_phase(&mut self, phase: u8, current: f32){
                self.$access.set_phase(phase, current);
            }
        }
    }
}
