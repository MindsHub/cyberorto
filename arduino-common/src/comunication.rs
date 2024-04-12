use core::{marker::PhantomData, pin::pin};

use futures::future::{select, Either};
use serde::{Deserialize, Serialize};
use serialmessage::{ParseState, SerMsg};

use crate::prelude::*;
/// comunication wrapper, it shouldn't be used directly
pub struct Comunication<Serial: AsyncSerial, Sleeper: Sleep> {
    /// phantom data
    ph: PhantomData<Sleeper>,
    /// how much time should I wait for a Byte to become available?
    ///
    /// Or How much time should I wait for a Byte to become Writable?
    timeout_us: u64,
    /// Serial interface
    serial: Serial,
    /// serial buffer
    input_buf: SerMsg,
    /// max incoming message size.
    buf: [u8; 50],
}

impl<Serial: AsyncSerial, Sleeper: Sleep> Comunication<Serial, Sleeper> {
    /// create a new Comunication Instance
    pub fn new(serial: Serial, timeout_us: u64) -> Self {
        Self {
            ph: PhantomData,
            timeout_us,
            serial,
            input_buf: SerMsg::new(),
            buf: [0u8; 50],
        }
    }
    /// try read a single byte. If waits more than timeout_us microseconds, then it returns None.
    async fn try_read_byte(&mut self) -> Option<u8> {
        match select(
            pin!(self.serial.read()),
            pin!(Sleeper::await_us(self.timeout_us)),
        )
        .await
        {
            Either::Left((b, _)) => Some(b),
            Either::Right(_) => None,
        }
    }
    /// try read a single byte.
    ///
    /// On success it returns true
    ///
    /// If waits more than timeout_us microseconds, then it returns false.
    async fn try_send_byte(&mut self, to_send: u8) -> bool {
        //let t = Select{ l: pin!(self.serial.write(to_send)), r: pin!(Sleeper::await_us(self.timeout_us)) };
        match select(
            pin!(self.serial.write(to_send)),
            pin!(Sleeper::await_us(self.timeout_us)),
        )
        .await
        {
            Either::Left(_) => true,
            Either::Right(_) => false,
        }
    }
    /// tries to read a complex message.
    ///
    /// On success returns the message and the corresponding Id
    ///
    /// On failure None
    pub async fn try_read<Out: for<'a> Deserialize<'a>>(&mut self) -> Option<(u8, Out)> {
        while let Some(b) = self.try_read_byte().await {
            let (state, _) = self.input_buf.parse_read_bytes(&[b]);
            if let ParseState::DataReady = state {
                let data = self.input_buf.return_read_data();
                let id = self.input_buf.return_msg_id();
                return Some((id, postcard::from_bytes(data).ok()?));
            }
        }
        None
    }
    ///tries to send a complex message.
    ///
    /// On success returns true, on timeout false.
    pub async fn send<Input: Serialize>(&mut self, to_send: Input, id: u8) -> bool {
        let Ok(msg) = postcard::to_slice(&to_send, &mut self.buf) else {
            return false;
        };
        let Some((buf, len)) = SerMsg::create_msg_arr(msg, id) else {
            return false;
        };
        //println!("bytes wide {}", len);
        for b in &buf[0..len] {
            if !self.try_send_byte(*b).await {
                return false;
            }
        }
        true
    }
}
