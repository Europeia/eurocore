use crate::core::error::Error;
use std::collections::{HashMap, VecDeque};
use std::ops::Add;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};
use tracing;

#[derive(Debug)]
enum Target {
    RecruitmentTelegram { sender: String },
    Telegram { sender: String },
    Restricted { sender: String },
    Standard,
}

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

#[derive(Clone)]
pub(crate) struct Sender {
    tx: mpsc::Sender<Command>,
}

impl Sender {
    async fn peek(&self, target: Target) -> Duration {
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

    async fn acquire(&self, target: Target) -> Result<(), Duration> {
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
    last_telegram: Option<Instant>,
    recruitment_cooldown: Duration,
    last_recruitment: Option<Instant>,
    restricted_action_cooldown: Duration,
    /// the last restrcted action performed by a given nation name, if any
    last_restricted_action: HashMap<String, Instant>,
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
            last_telegram: None,
            recruitment_cooldown,
            last_recruitment: None,
            restricted_action_cooldown,
            last_restricted_action: HashMap::new(),
        }
    }

    /// remove expired requests from bucket
    fn clean_bucket(&mut self) {
        let now = Instant::now();

        while let Some(&request) = self.requests.front() {
            if now.duration_since(request) > self.bucket_length {
                self.requests.pop_front();
            } else {
                break;
            }
        }
    }

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
    fn peek_recruitment(&mut self) -> Duration {
        if let Some(last_request) = self.last_recruitment {
            let now = Instant::now();

            if now.duration_since(last_request) > self.recruitment_cooldown {
                Duration::ZERO
            } else {
                self.recruitment_cooldown - now.duration_since(last_request)
            }
        } else {
            Duration::ZERO
        }
    }

    /// Naive method to check when next telegram can be sent. In this context, naive
    /// means that it does not take into account other limits that may prevent a telegram
    /// from being sent (e.g. the restricted action rate limit).
    fn peek_telegram(&mut self) -> Duration {
        if let Some(last_request) = self.last_telegram {
            let now = Instant::now();

            if now.duration_since(last_request) > self.telegram_cooldown {
                Duration::ZERO
            } else {
                self.telegram_cooldown - now.duration_since(last_request)
            }
        } else {
            Duration::ZERO
        }
    }

    /// Naive method to check when the next restricted action can be performed by a given nation.
    /// In this context, naive means that it does not take into account other limits that may
    /// prevent a restricted action from being performed (e.g. the standard rate limit).
    fn peek_restricted(&mut self, sender: &str) -> Duration {
        if let Some(last_request) = self.last_restricted_action.get(sender) {
            let now = Instant::now();

            if now.duration_since(*last_request) > self.restricted_action_cooldown {
                Duration::ZERO
            } else {
                self.restricted_action_cooldown - now.duration_since(*last_request)
            }
        } else {
            Duration::ZERO
        }
    }

    fn peek_standard(&mut self) -> Duration {
        self.clean_bucket();

        if self.requests.len() < self.max_requests {
            Duration::ZERO
        } else {
            self.bucket_length
                .saturating_sub(Instant::now().duration_since(*self.requests.front().unwrap()))
        }
    }

    fn acquire(&mut self, target: Target) -> Result<(), Duration> {
        let wait = self.peek(&target);

        let request_at = Instant::now().add(wait);

        match target {
            Target::RecruitmentTelegram { sender } => {
                self.last_recruitment = Some(request_at);

                self.last_telegram = Some(request_at);

                self.last_restricted_action
                    .insert(sender.to_string(), request_at);

                self.requests.push_back(request_at);
            }
            Target::Telegram { sender } => {
                self.last_telegram = Some(request_at);

                self.last_restricted_action
                    .insert(sender.to_string(), request_at);

                self.requests.push_back(request_at);
            }
            Target::Restricted { sender } => {
                self.last_restricted_action
                    .insert(sender.to_string(), request_at);

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

    fn process(&mut self, action: Action) -> Result<Response, Error> {
        match action {
            Action::Peek(target) => Ok(Response::Peek(self.peek(&target))),
            Action::Acquire(target) => Ok(Response::Acquire(self.acquire(target))),
            _ => Ok(Response::Ok),
        }
    }

    fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(e) => match e {
                    mpsc::error::TryRecvError::Empty => (),
                    mpsc::error::TryRecvError::Disconnected => {
                        tracing::warn!("rate limiter disconnected, exiting");
                        break;
                    }
                },
                Ok(command) => {
                    tracing::info!("command received");
                    let resp = self.process(command.action);
                    command.tx.send(resp.unwrap()).unwrap()
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
) -> (Sender, Receiver) {
    let (tx, rx) = mpsc::channel(16);

    (
        Sender { tx },
        Receiver::new(
            rx,
            max_requests,
            bucket_length,
            telegram_cooldown,
            recruitment_cooldown,
            restricted_action_cooldown,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, Instant};

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

        // Fill token bucket
        assert_eq!(limiter.acquire(Target::Standard), Ok(()));
        // Should now be ratelimited
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
        assert!(wait >= Duration::from_secs(4)); // or 5 depending on Instant resolution
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

        // All cooldowns should now be in effect
        let wait = limiter.peek(&Target::RecruitmentTelegram {
            sender: sender.clone(),
        });
        assert!(wait >= Duration::from_secs(14)); // Based on longest (recruitment = 15s)
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
        assert!(wait >= Duration::from_secs(19)); // 20s cooldown
    }
}
