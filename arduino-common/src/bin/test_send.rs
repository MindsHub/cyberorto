use arduino_common::{testable::Testable, Comunication};
use indicatif::ProgressIterator;
use rand::{thread_rng, Rng};

fn main(){
    let (master, slave) = Testable::new(0.03, 0.03);
    let mut master = Comunication::new(master);
    let mut slave = Comunication::new(slave);
    let mut rng = thread_rng();
    let mut corretti=0;
    let mut resend =0;
    let mut errori =0 ;
    for _ in (0..10000000).progress(){
        let mut to_send = [0u8; 10];
        rng.fill(&mut to_send[..]);
        loop{
            master.send(&to_send);
            //println!("\n");
            if let Some(x) = slave.read(){
                //println!("\n");
                if x==to_send{
                    corretti+=1;
                    break;
                }else{
                    errori+=1;
                    break;
                }
            }
            resend+=1;
        }
        
    }
    println!("corr={} resend={} errori={}", corretti, resend, errori);
    //Comunication{}.send();
}
