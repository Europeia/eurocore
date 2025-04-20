use crate::core::error::{ConfigError, Error};
use crate::ns::telegram::{Command, Header, Operation, Params, Response, Telegram, TgType};
use crate::sync::ratelimiter;
use crate::sync::ratelimiter::Target;
use crate::types::response;
use reqwest::{self, ClientBuilder};
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::Duration;
use tracing;

const MAX_COOLDOWN: Duration = Duration::from_millis(100);

#[derive(Debug)]
struct Client {
    url: String,
    client: reqwest::Client,
    key: String,
    recruitment_queue: VecDeque<Telegram>,
    standard_queue: VecDeque<Telegram>,
    limiter: ratelimiter::Sender,
    rx: mpsc::Receiver<Command>,
}

impl Client {
    fn new(
        user_agent: &str,
        url: &str,
        key: String,
        limiter: ratelimiter::Sender,
        rx: mpsc::Receiver<Command>,
    ) -> Result<Self, ConfigError> {
        let client = ClientBuilder::new().user_agent(user_agent).build()?;

        Ok(Self {
            url: url.to_owned(),
            client,
            key,
            recruitment_queue: VecDeque::new(),
            standard_queue: VecDeque::new(),
            limiter,
            rx,
        })
    }

    fn process_command(&mut self, command: Command) {
        let response = match command.operation {
            Operation::Queue(telegram) => {
                self.queue(telegram);
                Response::Ok
            }
            Operation::Delete(header) => {
                self.delete(header);
                Response::Ok
            }
            Operation::List => Response::List(self.list()),
        };

        if let Err(_) = command.tx.send(response) {
            tracing::error!("failed to send response");
        }
    }

    fn queue(&mut self, params: Params) {
        let telegram = Telegram::from_params(&self.key, params);

        match &telegram.tg_type {
            TgType::Standard => self.standard_queue.push_back(telegram),
            TgType::Recruitment => self.recruitment_queue.push_back(telegram),
        }
    }

    fn delete(&mut self, header: Header) {
        self.standard_queue
            .retain(|telegram| telegram.header() != header);

        self.recruitment_queue
            .retain(|telegram| telegram.header() != header);
    }

    fn list(&self) -> HashMap<String, Vec<response::Telegram>> {
        let mut response = HashMap::new();

        response.insert(
            "recruitment".to_string(),
            self.recruitment_queue
                .iter()
                .map(|tg| response::Telegram::new(&tg.recipient, &tg.telegram_id))
                .collect(),
        );

        response.insert(
            "standard".to_string(),
            self.standard_queue
                .iter()
                .map(|tg| response::Telegram::new(&tg.recipient, &tg.telegram_id))
                .collect(),
        );

        response
    }

    async fn try_send(&mut self) {
        if let Some(telegram) = self.get_telegram().await {
            if let Err(e) = self.send(telegram).await {
                tracing::error!("failed to send telegram: {}", e);
            }
        }
    }

    async fn get_telegram(&mut self) -> Option<Telegram> {
        for (index, telegram) in self.recruitment_queue.iter().enumerate() {
            if self
                .limiter
                .peek(ratelimiter::Target::RecruitmentTelegram {
                    sender: telegram.sender.clone(),
                })
                .await
                <= MAX_COOLDOWN
            {
                return Some(self.recruitment_queue.remove(index).unwrap());
            }
        }

        for (index, telegram) in self.standard_queue.iter().enumerate() {
            if self
                .limiter
                .peek(ratelimiter::Target::Telegram {
                    sender: telegram.sender.clone(),
                })
                .await
                <= MAX_COOLDOWN
            {
                return Some(self.standard_queue.remove(index).unwrap());
            }
        }

        None
    }

    async fn send(&mut self, telegram: Telegram) -> Result<(), Error> {
        let target = match &telegram.tg_type {
            TgType::Recruitment => Target::RecruitmentTelegram {
                sender: telegram.sender.clone(),
            },
            TgType::Standard => Target::Telegram {
                sender: telegram.sender.clone(),
            },
        };

        if let Err(duration) = self.limiter.acquire(target).await {
            tracing::info!("sleeping for {} ms", duration.as_millis());
            tokio::time::sleep(duration).await;
        }

        tracing::debug!("sending telegram");

        self.client
            .get(&self.url)
            .query(&telegram)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!("telegram client disconnected");
                    break;
                }
                Ok(command) => self.process_command(command),
            }

            self.try_send().await;

            tokio::time::sleep(MAX_COOLDOWN).await;
        }
    }
}
