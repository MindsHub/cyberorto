extern crate std;
use core::time::Duration;
use std::sync::Arc;

use super::{
    cyber::*,
    test_harness::{Dummy, TestMaster, Testable},
};

async fn init_test(timeout_us: u64) -> (TestMaster<Testable>, Slave<Testable, Dummy>) {
    let (master, slave) = Testable::new(0.0, 0.0);
    let master: TestMaster<Testable> = Master::new(master, timeout_us, 10);
    let slave: Slave<Testable, _> = Slave::new(slave, 10, b"ciao      ".clone(), Dummy::default());

    (master, slave)
}

#[tokio::test]
async fn test_led_set() {
    let (master, mut slave) = init_test(10).await;
    let led_state = slave.message_handler.led_state;
    assert!(!*led_state.lock().await);
    let _ = tokio::spawn(async move { slave.run().await });
    master.set_led(true).await.unwrap();
    assert!(*led_state.lock().await);
    master.set_led(false).await.unwrap();
    assert!(!*led_state.lock().await);
}
#[tokio::test]
async fn test_who_are_you() {
    let (master, mut slave) = init_test(10).await;
    let _ = tokio::spawn(async move { slave.run().await });
    let (name, version) = master.who_are_you().await.unwrap();
    assert_eq!(name, b"ciao      ".clone());
    assert_eq!(version, 0);
}
#[tokio::test]
async fn test_move_to() {
    let (master, mut slave) = init_test(10).await;
    let _ = tokio::spawn(async move { slave.run().await });
    master.move_to(0.0).await.unwrap();
}

#[tokio::test]
async fn test_blocking() {
    let (master, mut slave) = init_test(10000).await;
    let _ = tokio::spawn(async move { slave.run().await });
    let master = Arc::new(master);
    let m1 = master.clone();
    let q = tokio::spawn(async move { m1.move_to(1.0).await });
    tokio::time::sleep(Duration::from_millis(10)).await;
    let (name, version) = master.who_are_you().await.unwrap();
    assert_eq!(name, b"ciao      ".clone());
    assert_eq!(version, 0);
    assert!(!q.is_finished());
    let res = q.await.unwrap();
    assert_eq!(res, Ok(()))
}

#[tokio::test]
async fn test_timeout() {
    let (master, slave) = Testable::new(0.0, 1.0);
    let master: TestMaster<Testable> = Master::new(master, 10, 10);
    let mut slave: Slave<Testable, _> =
        Slave::new(slave, 10, b"ciao      ".clone(), Dummy::default());
    tokio::time::sleep(Duration::from_millis(10)).await;
    let _ = tokio::spawn(async move { slave.run().await });
    let ret = master.who_are_you().await;
    assert_eq!(ret, Err(()));
}
