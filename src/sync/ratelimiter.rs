use crate::core::error::Error;
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, Mul};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) enum Target {
    RecruitmentTelegram { sender: String },
    Telegram { sender: String },
    Restricted { sender: String },
    Standard,
}

impl Target {
    pub(crate) fn recruitment(sender: &str) -> Self {
        Self::RecruitmentTelegram {
            sender: sender.to_string(),
        }
    }

    pub(crate) fn telegram(sender: &str) -> Self {
        Self::Telegram {
            sender: sender.to_string(),
        }
    }

    pub(crate) fn restricted(sender: &str) -> Self {
        Self::Restricted {
            sender: sender.to_string(),
        }
    }
}

#[derive(Debug)]
enum Action {
    Peek(Target),
    Acquire(Target),
    Update,
}

struct Command {
    action: Action,
    tx: oneshot::Sender<Response>,
}

impl Command {
    fn new(action: Action, tx: oneshot::Sender<Response>) -> Self {
        Self { action, tx }
    }
}

#[derive(Debug)]
enum Response {
    Ok,
    Peek(Duration),
    Acquire(Result<(), Duration>),
}

#[derive(Clone, Debug)]
pub(crate) struct Sender {
    tx: mpsc::Sender<Command>,
}

impl Sender {
    #[tracing::instrument(skip_all)]
    pub(crate) async fn peek(&self, target: Target) -> Duration {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.tx.send(Command::new(Action::Peek(target), tx)).await {
            tracing::error!("Failed to send message: {}", e);
        };

        match rx.await {
            Ok(Response::Peek(duration)) => duration,
            Ok(_) => unreachable!(),
            Err(_) => unreachable!(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn acquire(&self, target: Target) -> Result<(), Duration> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self
            .tx
            .send(Command::new(Action::Acquire(target), tx))
            .await
        {
            tracing::error!("Failed to send message: {}", e);
        }

        match rx.await {
            Ok(Response::Acquire(result)) => result,
            Ok(_) => unreachable!(),
            Err(_) => unreachable!(),
        }
    }
}

pub(crate) struct Receiver {
    rx: mpsc::Receiver<Command>,
    max_requests: usize,
    bucket_length: Duration,
    requests: VecDeque<Instant>,
    telegram_cooldown: Duration,
    telegrams: VecDeque<Instant>,
    recruitment_cooldown: Duration,
    recruitment_telegrams: VecDeque<Instant>,
    restricted_action_cooldown: Duration,
    /// the last restrcted action performed by a given nation name, if any
    restricted_actions: HashMap<String, VecDeque<Instant>>,
}

impl Receiver {
    fn new(
        rx: mpsc::Receiver<Command>,
        max_requests: usize,
        bucket_length: Duration,
        telegram_cooldown: Duration,
        recruitment_cooldown: Duration,
        restricted_action_cooldown: Duration,
    ) -> Self {
        Self {
            rx,
            max_requests,
            bucket_length,
            requests: VecDeque::with_capacity(max_requests),
            telegram_cooldown,
            telegrams: VecDeque::new(),
            recruitment_cooldown,
            recruitment_telegrams: VecDeque::new(),
            restricted_action_cooldown,
            restricted_actions: HashMap::new(),
        }
    }

    /// remove expired requests from bucket
    #[tracing::instrument(skip_all)]
    fn clean_buckets(&mut self) {
        let now = Instant::now();

        self.requests
            .retain(|v| now.duration_since(*v) < self.bucket_length);

        self.telegrams
            .retain(|v| now.duration_since(*v) < self.telegram_cooldown);

        self.recruitment_telegrams
            .retain(|v| now.duration_since(*v) < self.recruitment_cooldown);

        for vec in self.restricted_actions.values_mut() {
            vec.retain(|&request| now.duration_since(request) < self.restricted_action_cooldown);
        }
    }

    #[tracing::instrument(skip_all)]
    fn peek(&mut self, target: &Target) -> Duration {
        let values = match target {
            Target::RecruitmentTelegram { sender } => {
                vec![
                    self.peek_recruitment(),
                    self.peek_telegram(),
                    self.peek_restricted(sender),
                    self.peek_standard(),
                ]
            }
            Target::Telegram { sender } => {
                vec![
                    self.peek_telegram(),
                    self.peek_restricted(sender),
                    self.peek_standard(),
                ]
            }
            Target::Restricted { sender } => {
                vec![self.peek_restricted(sender), self.peek_standard()]
            }
            Target::Standard => {
                vec![self.peek_standard()]
            }
        };

        values.into_iter().max().unwrap()
    }

    /// Naive method to check when next recruitment telegram can be sent. In this context, naive
    /// means that it does not take into account other limits that may prevent a recruitment telegram
    /// from being sent (e.g. the standard telegram rate limit).
    #[tracing::instrument(skip_all)]
    fn peek_recruitment(&mut self) -> Duration {
        self.clean_buckets();

        if self.recruitment_telegrams.is_empty() {
            Duration::ZERO
        } else {
            self.recruitment_cooldown
                .mul(self.recruitment_telegrams.len() as u32)
                .saturating_sub(
                    Instant::now()
                        .saturating_duration_since(*self.recruitment_telegrams.front().unwrap()),
                )
        }
    }

    /// Naive method to check when next telegram can be sent. In this context, naive
    /// means that it does not take into account other limits that may prevent a telegram
    /// from being sent (e.g. the restricted action rate limit).
    #[tracing::instrument(skip_all)]
    fn peek_telegram(&mut self) -> Duration {
        self.clean_buckets();

        if self.telegrams.is_empty() {
            Duration::ZERO
        } else {
            self.telegram_cooldown
                .mul(self.telegrams.len() as u32)
                .saturating_sub(
                    Instant::now().saturating_duration_since(*self.telegrams.front().unwrap()),
                )
        }
    }

    /// Naive method to check when the next restricted action can be performed by a given nation.
    /// In this context, naive means that it does not take into account other limits that may
    /// prevent a restricted action from being performed (e.g. the standard rate limit).
    #[tracing::instrument(skip_all)]
    fn peek_restricted(&mut self, sender: &str) -> Duration {
        self.clean_buckets();

        if let Some(bucket) = self.restricted_actions.get(sender) {
            if bucket.is_empty() {
                Duration::ZERO
            } else {
                self.restricted_action_cooldown
                    .mul(bucket.len() as u32)
                    .saturating_sub(
                        Instant::now().saturating_duration_since(*bucket.front().unwrap()),
                    )
            }
        } else {
            Duration::ZERO
        }
    }

    #[tracing::instrument(skip_all)]
    fn peek_standard(&mut self) -> Duration {
        self.clean_buckets();

        if self.requests.is_empty() {
            Duration::ZERO
        } else {
            self.bucket_length
                .mul((self.requests.len() / self.max_requests) as u32)
                .saturating_sub(Instant::now().duration_since(*self.requests.front().unwrap()))
        }
    }

    #[tracing::instrument(skip_all)]
    fn acquire(&mut self, target: Target) -> Result<(), Duration> {
        let wait = self.peek(&target);

        let request_at = Instant::now().add(wait);

        match target {
            Target::RecruitmentTelegram { sender } => {
                self.recruitment_telegrams.push_back(request_at);

                self.telegrams.push_back(request_at);

                self.restricted_actions
                    .entry(sender.to_string())
                    .or_default()
                    .push_back(request_at);

                self.requests.push_back(request_at);
            }
            Target::Telegram { sender } => {
                self.telegrams.push_back(request_at);

                self.restricted_actions
                    .entry(sender.to_string())
                    .or_default()
                    .push_back(request_at);

                self.requests.push_back(request_at);
            }
            Target::Restricted { sender } => {
                self.restricted_actions
                    .entry(sender.to_string())
                    .or_default()
                    .push_back(request_at);

                self.requests.push_back(request_at);
            }
            Target::Standard => {
                self.requests.push_back(request_at);
            }
        }

        if let Duration::ZERO = wait {
            Ok(())
        } else {
            Err(wait)
        }
    }

    #[tracing::instrument(skip_all)]
    fn process(&mut self, action: Action) -> Result<Response, Error> {
        match action {
            Action::Peek(target) => Ok(Response::Peek(self.peek(&target))),
            Action::Acquire(target) => Ok(Response::Acquire(self.acquire(target))),
            _ => Ok(Response::Ok),
        }
    }

    #[tracing::instrument(skip_all)]
    async fn run(&mut self) {
        loop {
            match self.rx.recv().await {
                None => {
                    tracing::warn!("channel is closed");
                    break;
                }
                Some(command) => {
                    let resp = self.process(command.action);

                    if let Err(_e) = command.tx.send(resp.unwrap()) {
                        tracing::error!("failed to send response")
                    }
                }
            }
        }
    }
}

pub(crate) fn new(
    max_requests: usize,
    bucket_length: Duration,
    telegram_cooldown: Duration,
    recruitment_cooldown: Duration,
    restricted_action_cooldown: Duration,
) -> Sender {
    let (tx, rx) = mpsc::channel(16);

    let sender = Sender { tx };

    let mut receiver = Receiver::new(
        rx,
        max_requests,
        bucket_length,
        telegram_cooldown,
        recruitment_cooldown,
        restricted_action_cooldown,
    );

    tokio::task::spawn(async move {
        receiver.run().await;
    });

    sender
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    fn make_receiver() -> Receiver {
        Receiver::new(
            mpsc::channel(1).1,
            2,
            Duration::from_secs(10),
            Duration::from_secs(5),
            Duration::from_secs(15),
            Duration::from_secs(20),
        )
    }

    #[test]
    fn test_standard_peek_and_acquire() {
        let mut limiter = make_receiver();

        assert_eq!(limiter.peek(&Target::Standard), Duration::ZERO);

        assert_eq!(limiter.acquire(Target::Standard), Ok(()));
        assert_eq!(limiter.peek(&Target::Standard), Duration::ZERO);

        assert_eq!(limiter.acquire(Target::Standard), Ok(()));
        let wait = limiter.peek(&Target::Standard);
        assert!(wait > Duration::ZERO);
    }

    #[test]
    fn test_telegram_peek_and_acquire() {
        let mut limiter = make_receiver();
        let sender = "test_sender".to_string();

        assert_eq!(
            limiter.peek(&Target::Telegram {
                sender: sender.clone()
            }),
            Duration::ZERO
        );
        assert_eq!(
            limiter.acquire(Target::Telegram {
                sender: sender.clone()
            }),
            Ok(())
        );

        let wait = limiter.peek(&Target::Telegram {
            sender: sender.clone(),
        });
        assert!(wait >= Duration::from_secs(4));
    }

    #[test]
    fn test_recruitment_telegram_peek_and_acquire() {
        let mut limiter = make_receiver();
        let sender = "recruiter".to_string();

        assert_eq!(
            limiter.acquire(Target::RecruitmentTelegram {
                sender: sender.clone()
            }),
            Ok(())
        );

        let wait = limiter.peek(&Target::RecruitmentTelegram {
            sender: sender.clone(),
        });
        assert!(wait >= Duration::from_secs(14));
    }

    #[test]
    fn test_restricted_action_peek_and_acquire() {
        let mut limiter = make_receiver();
        let sender = "nation".to_string();

        assert_eq!(
            limiter.acquire(Target::Restricted {
                sender: sender.clone()
            }),
            Ok(())
        );

        let wait = limiter.peek(&Target::Restricted {
            sender: sender.clone(),
        });
        assert!(wait >= Duration::from_secs(19));
    }
}
