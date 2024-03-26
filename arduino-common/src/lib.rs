//#![no_std]


use serialmessage::SerMsg;

pub trait Serial{
    fn write(&mut self, v: &[u8])->u8;
    fn read(&mut self)->u8;
}

#[repr(u8)]
enum Message{
    Send=0,
    Ack=1,
}



pub struct Comunication<S: Serial>{
    serial: S,
    
}
impl<S: Serial> Comunication<S>{
    pub fn new(serial: S)->Self{
        Self { serial }
    }

    pub fn send(&mut self, send_data: &[u8]){
        let (msg, len) = SerMsg::create_msg_arr(send_data, 1).unwrap();
        self.serial.write(&msg[..len]);
        
        /*println!("{:?}", msg[..len].to_vec());
        let mut next_msg = SerMsg::new();
        let (parse_state, len) = SerMsg::parse_read_bytes(&mut next_msg, &msg);
        match  parse_state {
            serialmessage::ParseState::Continue => todo!(),
            serialmessage::ParseState::DataReady => println!("{:?}", next_msg.return_read_data()),
            serialmessage::ParseState::CrcError => todo!(),
            serialmessage::ParseState::HighPayloadError => todo!(),
            serialmessage::ParseState::StopByteError => todo!(),
            serialmessage::ParseState::COBSError => todo!(),
        }*/
    }
    pub fn read(&mut self)->Option<[u8; 10]>{
        self.serial.read();
        None
    }
}

#[test]
fn test_send(){
    //Comunication{}.send();
    todo!()
}
