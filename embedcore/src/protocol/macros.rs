/// `self, lock, message => pattern => block...`
#[macro_export]
macro_rules! blocking_send {
    ($self:expr, $m:expr => $($p:pat => $block:block)+) => {{
        #[cfg(feature = "std")]
        let mut result: Result<Result<_, ()>, _> = Ok(Err(()));
        let mut lock = $self.inner.lock().await;
        for _ in 0..$self.resend_times {
            let future = async {
                if !lock.send($m.clone()).await {
                    return Err(());
                }
                defmt_or_log::debug!("blocking_send!: sent");

                while let Some((id_read, msg)) = lock.try_read::<Response>().await {
                    if id_read != lock.id {
                        continue;
                    }

                    match msg {
                        $crate::protocol::cyber::Response::Unsupported => {
                            defmt_or_log::error!("blocking_send!: received response Unsupported");
                            return Err(());
                        },
                        $crate::protocol::cyber::Response::Error(e) => {
                            defmt_or_log::error!("blocking_send!: received response Error({:?})", e);
                            return Err(());
                        }
                        $(
                            $p => $block
                        ),*
                        _ => {
                            defmt_or_log::error!("blocking_send!: received unexpected response {:?}", msg);
                            return Err(());
                        }
                    }
                }

                Err(())
            };
            #[cfg(feature = "std")]
            {
                result = tokio::time::timeout($self.timeout, future).await;

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
            #[cfg(not(feature = "std"))]
            {
                let r = future.await;

                if r.is_ok() {
                    defmt_or_log::debug!("blocking_send!: success");
                    return r;
                }
            }
        }

        #[cfg(feature = "std")]
        {
            match result {
                Ok(result) => result,
                Err(_) => Err(()),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Err(())
        }
    }};
}
