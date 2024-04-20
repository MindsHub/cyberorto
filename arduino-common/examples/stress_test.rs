use arduino_common::prelude::*;

struct Dummy;
impl MessagesHandler for Dummy {}

#[tokio::main]
async fn main() {
    let (master, slave) = Testable::new(0.2, 0.00);
    //let m = Box::leak(Box::new(Mutex::new(BotState::default())));
    let mut slave: Slave<Testable, tokio::time::Sleep, _> =
        Slave::new(slave, 0, b"ciao      ".clone(), Dummy);
    let q = tokio::spawn(async move { slave.run().await });
    let master: TestMaster<Testable> = Master::new(master, 5, 20);
    let mut ok = 0;
    let total = 10000;
    for _ in 0..total {
        if Ok((b"ciao      ".clone(), 0)) == master.who_are_you().await {
            ok += 1
        }
    }
    q.abort();
    println!("{ok}/{total}");
}
