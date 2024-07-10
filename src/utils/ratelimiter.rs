use std::collections::VecDeque;
use std::sync::Arc;
use tokio::time::{Instant, Duration, sleep};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct Ratelimiter {
    //TODO: create a method to update these from the headers that NS returns
    max_requests: usize,
    bucket_length: Duration,
    requests: Arc<Mutex<VecDeque<Instant>>>,
}

impl Ratelimiter {
    pub(crate) fn new() -> Self {
        Self {
            max_requests: 50,
            bucket_length: Duration::from_secs(30),
            requests: Arc::new(Mutex::new(VecDeque::new())),
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

            let cooldown = self.bucket_length - now
                .duration_since(*(*requests).front().unwrap());

            drop(requests);

            sleep(cooldown).await;
        }
    }
}