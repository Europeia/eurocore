use super::PERIOD;
use crate::core::error::{ConfigError, Error};
use crate::ns::telegram::{Command, Header, Operation, Params, Response, Telegram, TgType};
use crate::sync::ratelimiter;
use crate::sync::ratelimiter::Target;
use crate::types::response;
use reqwest::{self, ClientBuilder};
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct Client {
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

    #[tracing::instrument(skip_all)]
    fn process_command(&mut self, command: Command) {
        let response = match command.operation {
            Operation::Queue(telegrams) => {
                self.queue(telegrams);
                Response::Ok
            }
            Operation::Delete(header) => {
                self.delete(header);
                Response::Ok
            }
            Operation::List => Response::List(self.list()),
        };

        if command.tx.send(response).is_err() {
            tracing::error!("failed to send response");
        }
    }

    #[tracing::instrument(skip_all)]
    fn queue(&mut self, params: Vec<Params>) {
        for param in params {
            let telegram = Telegram::from_params(&self.key, param);

            match &telegram.tg_type {
                TgType::Standard => self.standard_queue.push_back(telegram),
                TgType::Recruitment => self.recruitment_queue.push_back(telegram),
            }
        }
    }

    #[tracing::instrument(skip_all)]
    fn delete(&mut self, header: Header) {
        self.standard_queue
            .retain(|telegram| telegram.header() != header);

        self.recruitment_queue
            .retain(|telegram| telegram.header() != header);
    }

    #[tracing::instrument(skip_all)]
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

    #[tracing::instrument(skip_all)]
    async fn try_send(&mut self) {
        if let Some(telegram) = self.get_telegram().await {
            if let Err(e) = self.send(telegram).await {
                tracing::error!("failed to send telegram: {}", e);
            }
        }
    }

    #[tracing::instrument(skip_all)]
    async fn get_telegram(&mut self) -> Option<Telegram> {
        for (index, telegram) in self.recruitment_queue.iter().enumerate() {
            if self
                .limiter
                .peek(ratelimiter::Target::RecruitmentTelegram {
                    sender: telegram.sender.clone(),
                })
                .await
                <= PERIOD
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
                <= PERIOD
            {
                return Some(self.standard_queue.remove(index).unwrap());
            }
        }

        None
    }

    #[tracing::instrument(skip_all)]
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

    #[tracing::instrument(skip_all)]
    pub(crate) async fn run(&mut self) {
        let mut interval = tokio::time::interval(PERIOD);

        loop {
            tokio::select! {
                Some(command) = self.rx.recv() => {
                    self.process_command(command);
                }

                _  = interval.tick() => {
                    self.try_send().await;
                }

            }
        }
    }
}

pub(crate) fn new(
    user_agent: &str,
    url: &str,
    key: String,
    limiter: ratelimiter::Sender,
) -> Result<(mpsc::Sender<Command>, Client), ConfigError> {
    let (tx, rx) = mpsc::channel(16);

    let client = Client::new(user_agent, url, key, limiter, rx)?;

    Ok((tx, client))
}
