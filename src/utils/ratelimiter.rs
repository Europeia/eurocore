use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug)]
pub(crate) enum Target<'a> {
    Standard,
    Restricted(&'a str),
    Telegram(&'a str),
    RecruitmentTelegram(&'a str),
}

#[derive(Clone, Debug)]
pub(crate) struct Ratelimiter {
    //TODO: create a method to update these from the headers that NS returns
    max_requests: usize,
    bucket_length: Duration,
    requests: Arc<RwLock<VecDeque<Instant>>>,

    telegram_cooldown: Duration,
    last_telegram: Arc<RwLock<Option<Instant>>>,
    recruitment_cooldown: Duration,
    last_recruitment: Arc<RwLock<Option<Instant>>>,

    // 'communication actions' like creating dispatches, sending telegrams, and
    // posting on RMBs are subject to an additional ratelimit of ~1 action per
    // 30 seconds per nation
    restricted_action_cooldown: Duration,
    last_restricted_action: Arc<RwLock<HashMap<String, Instant>>>,
}

impl Ratelimiter {
    pub(crate) fn new(
        max_requests: usize,
        bucket_length: Duration,
        telegram_cooldown: Duration,
        recruitment_telegram_cooldown: Duration,
        restricted_action_cooldown: Duration,
    ) -> Self {
        assert!(max_requests > 0);

        Self {
            max_requests,
            bucket_length,
            telegram_cooldown,
            recruitment_cooldown: recruitment_telegram_cooldown,
            requests: Arc::new(RwLock::new(VecDeque::new())),
            last_telegram: Arc::new(RwLock::new(None)),
            last_recruitment: Arc::new(RwLock::new(None)),
            last_restricted_action: Arc::new(RwLock::new(HashMap::new())),
            restricted_action_cooldown,
        }
    }

    /// remove expired requests from the bucket
    async fn clear_bucket(&self) {
        let mut requests = self.requests.write().await;
        let now = Instant::now();

        while let Some(&request) = requests.front() {
            if now.duration_since(request) > self.bucket_length {
                (*requests).pop_front();
            } else {
                break;
            }
        }
    }

    /// Returns the time until the `target` action can be performed
    pub(crate) async fn peek_ratelimit(&self, target: Target<'_>) -> Duration {
        let values = match target {
            Target::Standard => vec![self.peek().await],
            Target::Restricted(nation) => {
                vec![self.peek().await, self.peek_restricted(nation).await]
            }
            Target::Telegram(nation) => {
                vec![
                    self.peek().await,
                    self.peek_restricted(nation).await,
                    self.peek_telegram().await,
                ]
            }
            Target::RecruitmentTelegram(nation) => {
                vec![
                    self.peek().await,
                    self.peek_restricted(nation).await,
                    self.peek_telegram().await,
                    self.peek_recruitment_telegram().await,
                ]
            }
        };

        *(values.iter().max().unwrap())
    }

    async fn peek(&self) -> Duration {
        self.clear_bucket().await;

        let requests = self.requests.read().await;

        if requests.len() < self.max_requests {
            Duration::ZERO
        } else {
            let now = Instant::now();
            self.bucket_length - now.duration_since(*requests.front().unwrap())
        }
    }

    async fn peek_restricted(&self, nation: &str) -> Duration {
        if let Some(instant) = self.last_restricted_action.read().await.get(nation) {
            let now = Instant::now();
            self.restricted_action_cooldown - now.duration_since(*instant)
        } else {
            Duration::ZERO
        }
    }

    async fn peek_telegram(&self) -> Duration {
        if let Some(instant) = *self.last_telegram.read().await {
            let now = Instant::now();
            self.telegram_cooldown - now.duration_since(instant)
        } else {
            Duration::ZERO
        }
    }

    async fn peek_recruitment_telegram(&self) -> Duration {
        if let Some(instant) = *self.last_recruitment.read().await {
            let now = Instant::now();
            self.recruitment_cooldown - now.duration_since(instant)
        } else {
            Duration::ZERO
        }
    }

    pub(crate) async fn acquire_for(&self, target: Target<'_>) {
        match target {
            Target::Standard => self.acquire().await,
            Target::Restricted(nation) => self.acquire_for_restricted(nation).await,
            Target::Telegram(nation) => self.acquire_for_telegram(nation).await,
            Target::RecruitmentTelegram(nation) => self.acquire_for_recruitment(nation).await,
        }
    }

    async fn acquire(&self) {
        loop {
            tracing::debug!("Acquiring for standard action");
            self.clear_bucket().await;
            let now = Instant::now();

            let mut requests = self.requests.write().await;

            if requests.len() < self.max_requests {
                requests.push_back(now);
                return;
            }

            let cooldown = self.bucket_length - now.duration_since(*requests.front().unwrap());

            drop(requests);

            sleep(cooldown).await;
        }
    }

    async fn acquire_for_restricted(&self, nation: &str) {
        loop {
            tracing::debug!("Acquiring for restricted action");
            let now = Instant::now();
            let last_restricted_action = self.last_restricted_action.read().await;

            if let Some(last) = last_restricted_action.get(nation) {
                if now.duration_since(*last) < self.restricted_action_cooldown {
                    let cooldown = self.restricted_action_cooldown - now.duration_since(*last);
                    drop(last_restricted_action);
                    sleep(cooldown).await;
                    continue;
                }
            }

            drop(last_restricted_action);
            let mut last_restricted_action = self.last_restricted_action.write().await;

            last_restricted_action.insert(nation.to_string(), now);

            self.acquire().await;
            return;
        }
    }

    async fn acquire_for_telegram(&self, nation: &str) {
        loop {
            tracing::debug!("Acquiring for telegram");
            let now = Instant::now();
            let mut last_telegram_sent = self.last_telegram.write().await;

            if let Some(last) = *last_telegram_sent {
                if now.duration_since(last) < self.telegram_cooldown {
                    drop(last_telegram_sent);
                    sleep(self.telegram_cooldown - now.duration_since(last)).await;
                    continue;
                }
            }

            *last_telegram_sent = Some(now);

            self.acquire_for_restricted(nation).await;
            return;
        }
    }

    async fn acquire_for_recruitment(&self, nation: &str) {
        loop {
            tracing::debug!("Acquiring for recruitment telegram");
            let now = Instant::now();
            let mut last_recruitment_telegram_sent = self.last_recruitment.write().await;

            if let Some(last) = *last_recruitment_telegram_sent {
                if now.duration_since(last) < self.recruitment_cooldown {
                    drop(last_recruitment_telegram_sent);
                    sleep(self.recruitment_cooldown - now.duration_since(last)).await;
                    continue;
                }
            }

            *last_recruitment_telegram_sent = Some(now);

            self.acquire_for_telegram(nation).await;
            return;
        }
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
            Duration::from_secs(10),
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
            Duration::from_secs(10),
        );

        let start = Instant::now();
        ratelimiter.acquire_for_telegram("upc").await;

        ratelimiter.acquire_for_telegram("upc").await;
        assert!(start.elapsed() >= Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_ratelimiter_for_recruitment_telegram() {
        let ratelimiter = Ratelimiter::new(
            50,
            Duration::from_secs(10),
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(10),
        );

        let start = Instant::now();
        ratelimiter.acquire_for_recruitment("upc").await;

        ratelimiter.acquire_for_telegram("upc").await;
        assert!(start.elapsed() >= Duration::from_secs(10));

        ratelimiter.acquire_for_recruitment("upc").await;
        assert!(start.elapsed() >= Duration::from_secs(30));
    }
}
