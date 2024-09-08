use std::{collections::HashMap, sync::Mutex, time::Duration};

use ratelimit::Ratelimiter;

#[derive(Default)]
pub(crate) struct Limiter {
    inner: Mutex<HashMap<i64, Ratelimiter>>,
}

impl Limiter {
    pub(crate) fn wait(&self, user_id: i64) -> Result<(), Duration> {
        self.inner
            .lock()
            .unwrap()
            .entry(user_id)
            .or_insert_with(|| {
                Ratelimiter::builder(30, Duration::from_secs(60))
                    .initial_available(30)
                    .max_tokens(120)
                    .build()
                    .expect("Ratelimiter with proper settings")
            })
            .try_wait()
    }
}
