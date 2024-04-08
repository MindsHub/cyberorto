use core::{marker::PhantomData, pin::pin};

use futures::future::{select, Either};
use serde::{Deserialize, Serialize};
use serialmessage::{ParseState, SerMsg};

use crate::prelude::*;
pub struct Comunication<Serial: AsyncSerial, Sleeper: Sleep> {
    ph: PhantomData<Sleeper>,
    timeout_us: u64,
    serial: Serial,
    input_buf: SerMsg,
    buf: [u8; 20],
}


impl<Serial: AsyncSerial, Sleeper: Sleep> Comunication<Serial, Sleeper> {
    pub fn new(serial: Serial, timeout_us: u64) -> Self {
        Self {
            ph: PhantomData,
            timeout_us,
            serial,
            input_buf: SerMsg::new(),
            buf: [0u8; 20],
        }
    }
    async fn try_read_byte(&mut self) -> Option<u8> {
        //let t = Select{ l: pin!(self.serial.read()), r: pin!(Sleeper::await_us(self.timeout_us)) };
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