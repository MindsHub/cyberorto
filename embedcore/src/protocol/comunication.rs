
use defmt_or_log::{error, trace};
use serde::{Deserialize, Serialize};
use serialmessage::{ParseState, SerMsg};

use super::AsyncSerial;
/// Comunication wrapper, it shouldn't be used directly.
/// Note that this will not do any timeout.
pub struct Comunication<Serial: AsyncSerial> {
    /// Serial interface
    pub serial: Serial,
    /// serial buffer
    input_buf: SerMsg,
    /// max incoming message size.
    buf: [u8; 50],
}

pub enum CommunicationError {
    InvalidMsg,
    CantRead,
    PostcardError(postcard::Error),
}

// TODO timeout shouldn't be handled here anymore, remove every reference to it
impl<Serial: AsyncSerial> Comunication<Serial> {
    /// create a new Comunication Instance
    pub fn new(serial: Serial) -> Self {
        Self {
            serial,
            input_buf: SerMsg::new(),
            buf: [0u8; 50],
        }
    }
    /// try read a single byte. If waits more than timeout_us microseconds, then it returns None.
    pub async fn try_read_byte(&mut self) -> Option<u8> {
        trace!("try_read_byte() called");
        Some(self.serial.read().await)
        // TODO maybe use select? Shouldn't be needed though
        /*match select(
            pin!(self.serial.read()),
            pin!(Timer::after_micros(self.timeout_us)),
        )
        .await
        {
            Either::First(b) => Some(b),
            Either::Second(_) => None,
        }*/
    }
    /// try read a single byte.
    ///
    /// On success it returns true
    ///
    /// If waits more than timeout_us microseconds, then it returns false.
    async fn try_send_byte(&mut self, to_send: u8) -> bool {
        trace!("try_send_byte() {to_send}");
        self.serial.write(to_send).await;
        true
        // TODO maybe use select? Shouldn't be needed though
        /*match select(
            pin!(self.serial.write(to_send)),
            pin!(Timer::after_micros(self.timeout_us)),
        )
        .await
        {
            Either::First(_) => true,
            Either::Second(_) => false,
        }*/
    }
    /// tries to read a complex message.
    ///
    /// On success returns the message and the corresponding Id
    ///
    /// On failure None
    pub async fn try_read<Out: for<'a> Deserialize<'a>>(&mut self) -> Result<(u8, Out), CommunicationError> {
        trace!("try_read() called");
        while let Some(b) = self.try_read_byte().await {
            trace!("try_read() got {b}");
            let (state, _) = self.input_buf.parse_read_bytes(&[b]);
            if let ParseState::DataReady = state {
                let data = self.input_buf.return_read_data();
                let id = self.input_buf.return_msg_id();
                //return Ok((id, postcard::from_bytes(&[0]).unwrap()));
                return postcard::from_bytes(data)
                    .map(|m| (id, m))
                    .map_err(CommunicationError::PostcardError);
            }
        }
        Err(CommunicationError::CantRead)
    }
    ///tries to send a complex message.
    ///
    /// On success returns true, on timeout false.
    pub async fn send<Input: Serialize>(&mut self, to_send: Input, id: u8) -> bool {
        let Ok(msg) = postcard::to_slice(&to_send, &mut self.buf) else {
            error!("send(): postcard::to_slice failed");
            return false;
        };
        let Some((buf, len)) = SerMsg::create_msg_arr(msg, id) else {
            error!("send(): SerMsg::create_msg_arr failed");
            return false;
        };
        trace!("send(): sending bytes one by one");
        for b in &buf[0..len] {
            if !self.try_send_byte(*b).await {
                error!("send(): self.try_send_byte failed");
                return false;
            }
        }
        trace!("send(): exiting");
        true
    }
}
