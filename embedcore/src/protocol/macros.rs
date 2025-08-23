// self, lock, message: pattern => block...
#[macro_export]
macro_rules! blocking_send {
    ($self:expr, $lock:expr, $m:expr => $($p:pat => $block:block),+) => {{
        let mut result: Result<Result<_, ()>, _> = Ok(Err(()));
        for _ in 0..$self.resend_times {
            result = tokio::time::timeout(
                $self.timeout,
                async {
                    if !$lock.as_mut().unwrap().send($m.clone()).await {
                        return Err(());
                    }
                    defmt_or_log::debug!("blocking_send!: sent");

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

                    Err(())
                }
            ).await;

            if let Ok(r) = result {
                if r.is_ok() {
                    defmt_or_log::debug!("blocking_send!: success");
                    return r;
                } else {
                    result = Ok(r);
                }
            }
            defmt_or_log::debug!("blocking_send!: timeout");
        }

        match result {
            Ok(result) => result,
            Err(_) => Err(()),
        }
    }};
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
