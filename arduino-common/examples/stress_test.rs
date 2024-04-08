use arduino_common::prelude::*;

#[tokio::main]
async fn main() {
    let (master, slave) = Testable::new(0.1, 0.00);
    let mut slave: Slave<Testable, StdSleeper> = Slave::new(slave, 0, b"ciao      ".clone());
    let q = tokio::spawn(async move { slave.run().await });
    let mut master: TestMaster<Testable> = Master::new(master, 5);
    let mut ok = 0;
    let total = 10000;
    for _ in 0..total {
        if Ok((b"ciao      ".clone(), 0)) == master.who_are_you().await {
            //println!("OK");
            ok += 1
        } else {
            //println!("NO");
        }
    }
    q.abort();
    println!("{ok}/{total}");
}