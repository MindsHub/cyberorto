use arduino_common::prelude::*;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let (master, slave) = Testable::new(0.2, 0.00);
    let m = Box::leak(Box::new(Mutex::new(BotState::default())));
    let mut slave: SlaveBot<Testable, StdSleeper, _> =
        SlaveBot::new(slave, 0, b"ciao      ".clone(), m);
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
