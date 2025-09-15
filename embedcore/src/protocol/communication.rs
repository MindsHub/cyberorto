#[cfg(feature = "std")]
extern crate std;

use defmt_or_log::{error, trace};
use serde::{Deserialize, Serialize};
use serialmessage::{ParseState, SerMsg};

use crate::protocol::cyber::Response;

use super::AsyncSerial;
/// Communication wrapper, it shouldn't be used directly.
/// Note that this will not do any timeout.
pub struct Communication<Serial: AsyncSerial> {
    /// Serial interface
    pub serial: Serial,
    /// serial buffer
    input_buf: SerMsg,
    /// max incoming message size.
    buf: [u8; 50],
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CommunicationError {
    ReadShouldReturnOneByte {
        actual_byte_count_read: usize,
        buffer_content: u8,
    },
    ReadError {
        #[cfg(feature = "std")]
        #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
        error: std::io::Error,
        #[cfg(feature = "ch32")]
        #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
        error: ch32_hal::usart::Error,
        buffer_content: u8,
    },
    WriteShouldReturnOneByte {
        actual_byte_count_written: usize,
        buffer_content: u8,
    },
    WriteError {
        #[cfg(feature = "std")]
        #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
        error: std::io::Error,
        #[cfg(feature = "ch32")]
        #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
        error: ch32_hal::usart::Error,
        buffer_content: u8,
    },
    PostcardError(
        #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
        postcard::Error
    ),
    Timeout,
    SerMsgError,
    UnsupportedResponse,
    ErrorResponse([u8; 10]),
    MismatchedResponse(Response),
}

// TODO timeout shouldn't be handled here anymore, remove every reference to it
impl<Serial: AsyncSerial> Communication<Serial> {
    /// create a new Communication Instance
    pub fn new(serial: Serial) -> Self {
        Self {
            serial,
            input_buf: SerMsg::new(),
            buf: [0u8; 50],
        }
    }
    /// tries to read a complex message.
    ///
    /// On success returns the message and the corresponding Id
    ///
    /// On failure a CommunicationError
    pub async fn try_read<Out: for<'a> Deserialize<'a>>(&mut self) -> Result<(u8, Out), CommunicationError> {
        trace!("try_read() called");
        loop {
            let b = self.serial.read().await?;
            trace!("try_read() read byte {}", b);
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
    }
    ///tries to send a complex message.
    pub async fn send<Input: Serialize>(&mut self, to_send: Input, id: u8) -> Result<(), CommunicationError> {
        let msg = postcard::to_slice(&to_send, &mut self.buf).map_err(CommunicationError::PostcardError)?;
        let Some((buf, len)) = SerMsg::create_msg_arr(msg, id) else {
            error!("send(): SerMsg::create_msg_arr failed");
            return Err(CommunicationError::SerMsgError);
        };
        trace!("send(): sending bytes one by one");
        for b in &buf[0..len] {
            self.serial.write(*b).await?
        }
        Ok(())
    }
}
