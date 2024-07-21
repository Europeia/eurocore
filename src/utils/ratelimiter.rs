use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration, Instant};
use tracing;

#[derive(Clone, Debug)]
pub(crate) struct Ratelimiter {
    //TODO: create a method to update these from the headers that NS returns
    max_requests: usize,
    bucket_length: Duration,
    telegram_cooldown: Duration,
    recruitment_telegram_cooldown: Duration,
    requests: Arc<Mutex<VecDeque<Instant>>>,
    last_telegram_sent: Arc<RwLock<Option<Instant>>>,
    last_recruitment_telegram_sent: Arc<RwLock<Option<Instant>>>,
}

impl Ratelimiter {
    pub(crate) fn new(
        max_requests: usize,
        bucket_length: Duration,
        telegram_cooldown: Duration,
        recruitment_telegram_cooldown: Duration,
    ) -> Self {
        Self {
            max_requests,
            bucket_length,
            telegram_cooldown,
            recruitment_telegram_cooldown,
            requests: Arc::new(Mutex::new(VecDeque::new())),
            last_telegram_sent: Arc::new(RwLock::new(None)),
            last_recruitment_telegram_sent: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) async fn acquire(&self) {
        loop {
            let mut requests = self.requests.lock().await;
            let now = Instant::now();

            while let Some(&request) = requests.front() {
                if now.duration_since(request) > self.bucket_length {
                    (*requests).pop_front();
                } else {
                    break;
                }
            }

            if (*requests).len() < self.max_requests {
                (*requests).push_back(now);
                return;
            }

            let cooldown = self.bucket_length - now.duration_since(*(*requests).front().unwrap());

            drop(requests);

            sleep(cooldown).await;
        }
    }

    pub(crate) async fn acquire_for_telegram(&self) {
        tracing::debug!("Starting acquire_for_telegram loop");
        loop {
            let cooldown_finished = {
                let last_telegram_sent = self.last_telegram_sent.read().await;
                let now = Instant::now();
                match *last_telegram_sent {
                    Some(last) => now.duration_since(last) >= self.telegram_cooldown,
                    None => true,
                }
            };

            if cooldown_finished {
                tracing::debug!("Acquiring ratelimiter for telegram");
                self.acquire().await;
                let now = Instant::now();
                let mut last_standard_message = self.last_telegram_sent.write().await;
                *last_standard_message = Some(now);
                return;
            } else {
                tracing::debug!("Cooldown not finished, sleeping");
                let last_telegram_sent = self.last_telegram_sent.read().await;
                let now = Instant::now();
                let elapsed = now.duration_since(last_telegram_sent.unwrap());
                drop(last_telegram_sent);
                sleep(self.telegram_cooldown - elapsed).await;
            }
        }
    }

    pub(crate) async fn acquire_for_recruitment_telegram(&self) {
        tracing::debug!("Starting acquire_for_recruitment_telegram loop");
        loop {
            let can_send_recruitment = {
                let last_recruitment_message = self.last_recruitment_telegram_sent.read().await;
                let now = Instant::now();
                match *last_recruitment_message {
                    Some(last) => now.duration_since(last) >= self.recruitment_telegram_cooldown,
                    None => true,
                }
            };

            if can_send_recruitment {
                tracing::debug!("Acquiring ratelimiter for recruitment telegram");
                self.acquire_for_telegram().await;
                let now = Instant::now();
                let mut last_recruitment_message =
                    self.last_recruitment_telegram_sent.write().await;
                *last_recruitment_message = Some(now);
                return;
            } else {
                tracing::debug!("Cooldown not finished, sleeping");
                let last_recruitment_message = self.last_recruitment_telegram_sent.read().await;
                let now = Instant::now();
                let elapsed = now.duration_since(last_recruitment_message.unwrap());
                drop(last_recruitment_message);
                sleep(self.recruitment_telegram_cooldown - elapsed).await;
            }
        }
    }

    pub(crate) async fn get_last_telegram_timestamp(&self) -> Option<Instant> {
        let last_telegram_sent = self.last_telegram_sent.read().await;
        *last_telegram_sent
    }

    pub(crate) fn telegram_cooldown(&self) -> Duration {
        self.telegram_cooldown
    }

    pub(crate) async fn get_last_recruitment_telegram_timestamp(&self) -> Option<Instant> {
        let last_recruitment_telegram_sent = self.last_recruitment_telegram_sent.read().await;
        *last_recruitment_telegram_sent
    }

    pub(crate) fn recruitment_telegram_cooldown(&self) -> Duration {
        self.recruitment_telegram_cooldown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ratelimiter() {
        let ratelimiter = Ratelimiter::new(
            50,
            Duration::from_secs(10),
            Duration::from_secs(10),
            Duration::from_secs(30),
        );

        for _ in 0..50 {
            ratelimiter.acquire().await;
        }

        let start = Instant::now();
        ratelimiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_ratelimiter_for_telegram() {
        let ratelimiter = Ratelimiter::new(
            50,
            Duration::from_secs(10),
            Duration::from_secs(10),
            Duration::from_secs(30),
        );

        let start = Instant::now();
        ratelimiter.acquire_for_telegram().await;

        ratelimiter.acquire_for_telegram().await;
        assert!(start.elapsed() >= Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_ratelimiter_for_recruitment_telegram() {
        let ratelimiter = Ratelimiter::new(
            50,
            Duration::from_secs(10),
            Duration::from_secs(10),
            Duration::from_secs(30),
        );

        let start = Instant::now();
        ratelimiter.acquire_for_recruitment_telegram().await;

        ratelimiter.acquire_for_telegram().await;
        assert!(start.elapsed() >= Duration::from_secs(10));

        ratelimiter.acquire_for_recruitment_telegram().await;
        assert!(start.elapsed() >= Duration::from_secs(30));
    }
}
