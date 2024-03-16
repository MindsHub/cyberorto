//#![no_std]


use serialmessage::SerMsg;

trait Serial{

}

pub struct Comunication{

    
}
impl Comunication{
    pub fn send(&mut self){
        let send_data_vec= [1, 2, 3, 4];
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

#[test]
fn test_send(){
    Comunication{}.send();
    todo!()
}
