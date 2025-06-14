// self, lock, message: pattern => block...
#[macro_export]
macro_rules! blocking_send {
    ($self:ident, $lock:ident, $m:ident : $($p:pat => $block:block),+) => {

        for _ in 0..$self.resend_times {
            // send Move
            if !$lock.as_mut().unwrap().send($m.clone()).await {
                continue;
            }
            //let id = $lock.as_mut().unwrap().id;

            while let Some((id_read, msg)) = $lock.as_mut().unwrap().try_read::<Response>().await {
                if id_read != $lock.as_mut().unwrap().id {
                    continue;
                }

                match msg {
                    $(
                        $p => $block
                    ),*
                }
            }
        }
    };
}
/// wait(self, lock, ms)
#[macro_export]
macro_rules! wait {
    ($self:ident, $lock:ident, $ms:ident) => {
        $lock.take();
        embassy_time::Timer::after_millis($ms).await;

        $lock = Some($self.inner.lock().await);
        if !$lock.as_mut().unwrap().send(Message::Poll).await {
            continue;
        }
    };
}
