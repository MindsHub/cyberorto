//#![no_std]


use serialmessage::SerMsg;

pub trait Serial{

}
enum Message{
    Move(f32, f32, f32),
    Plow(f32),
    Arrived(),
    Ended(),
}

pub struct Comunication<S: Serial>{
    s: S,
}
impl<S: Serial> Comunication<S>{
    pub fn try_receive(&mut  self)->Option<()>{
        todo!()
    }

    pub fn send(&mut self){
        let send_data_vec= [1, 2, 3, 4];
        //SerMsg::;
        let (msg, len) = SerMsg::create_msg_arr(&send_data_vec, 1).unwrap();
        println!("{:?}", msg[..len].to_vec());
        let mut next_msg = SerMsg::new();
        let (parse_state, len) = SerMsg::parse_read_bytes(&mut next_msg, &msg);
        match  parse_state {
            serialmessage::ParseState::Continue => todo!(),
            serialmessage::ParseState::DataReady => println!("{:?}", next_msg.return_read_data()),
            serialmessage::ParseState::CrcError => todo!(),
            serialmessage::ParseState::HighPayloadError => todo!(),
            serialmessage::ParseState::StopByteError => todo!(),
            serialmessage::ParseState::COBSError => todo!(),
        }
        
    }
}

struct StupidSerial;
impl Serial for StupidSerial{}

#[test]
fn test_send(){
    Comunication{s: StupidSerial}.send();
    todo!()
}
