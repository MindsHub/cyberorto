use std::{borrow::Cow, thread::sleep, time::Duration};

use rand::{thread_rng, Rng};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use serialport::{ClearBuffer, SerialPort};
fn flush(port: &mut Box<dyn SerialPort>){
    port.flush().unwrap();
    let to_read = port.bytes_to_read().unwrap();
    if to_read==0{
        return;
    }
    let mut buf: Vec<u8> = vec![0u8; to_read as usize];
    port.read_exact(buf.as_mut_slice()).unwrap();
    port.clear(ClearBuffer::Input).unwrap();
    port.clear(ClearBuffer::Output).unwrap();
    
}
fn main(){
    let mut port = serialport::new("/dev/ttyACM0", 115200)
    .timeout(Duration::from_millis(100))
    .parity(serialport::Parity::None)
    .stop_bits(serialport::StopBits::One)
    .flow_control(serialport::FlowControl::None)
    .open().expect("Failed to open port");
    
    flush(&mut port);
    //flush(&mut port);
    //flush(&mut port);
    let mut rng = thread_rng();
    const len: usize = 4000;
    const ITER: usize= 100000;
    let mut to_send = [0u8; len];
    
    let mut corretti=0;
    let mut errori=0;
    
    sleep(Duration::from_secs_f32(0.5));
    let _ = port.read(&mut to_send);
    let _ = port.read(&mut to_send);
    let _ = port.read(&mut to_send);
    let pb = ProgressBar::new((len*ITER) as u64 );
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
    let mut cur =0;
    for _i in (0..100000){
        rng.fill(&mut to_send[..]);

        port.write_all(&to_send[..]).unwrap();
        port.flush().unwrap();
        cur+=len as u64;
        pb.set_position(cur);
        //sleep(Duration::from_millis(1));
        let mut read = [0u8; len];
        match port.read_exact(&mut read){
            Ok(()) => {
                /*if len<1024{
                println!("sent {}, recv {}", to_send.len(), len);
                errori+=1;
                continue;
                }*/
                if read == to_send{
                    corretti+=1;
                }else{
                    println!("wrong");
                    errori+=1;
                }
            }
            Err(x) => {
                println!("{x}");
            },
            
        }
    }
    println!("{} {}", corretti, errori);

}