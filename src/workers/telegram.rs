use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;

use crate::core::client::Client;
use crate::core::error::ConfigError;
use crate::ns::telegram::{Command, Header, Operation, Params, Response, Telegram, TgType};
use crate::types::response;
use crate::utils::ratelimiter::Target;
use crate::workers::PERIOD;

#[derive(Debug)]
pub(crate) struct TelegramClient {
    client: Client,
    client_key: String,
    recruitment_queue: VecDeque<Telegram>,
    standard_queue: VecDeque<Telegram>,

    rx: mpsc::Receiver<Command>,
}

impl TelegramClient {
    pub(crate) fn new(
        client: Client,
        client_key: String,
        rx: mpsc::Receiver<Command>,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            client,
            client_key,
            recruitment_queue: VecDeque::new(),
            standard_queue: VecDeque::new(),
            rx,
        })
    }

    fn queue(&mut self, params: Params) {
        let telegram = Telegram::from_params(&self.client_key, params);

        let queue = match telegram.tg_type {
            TgType::Recruitment => &mut self.recruitment_queue,
            TgType::Standard => &mut self.standard_queue,
        };

        queue.push_back(telegram);
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

    async fn send(&mut self) {
        if let Some(telegram) = self.get_tg().await {
            match self.client.send_telegram(telegram).await {
                Ok(_) => {
                    tracing::info!("Telegram sent");
                }
                Err(e) => {
                    tracing::error!("Error sending telegram, {:?}", e);
                }
            };
        }
    }

    async fn get_tg(&mut self) -> Option<Telegram> {
        for (index, tg) in self.recruitment_queue.iter().enumerate() {
            if self
                .client
                .ratelimiter
                .peek_ratelimit(Target::RecruitmentTelegram(&tg.sender))
                .await
                <= PERIOD
            {
                return Some(self.recruitment_queue.remove(index).unwrap());
            }
        }

        for (index, tg) in self.standard_queue.iter().enumerate() {
            if self
                .client
                .ratelimiter
                .peek_ratelimit(Target::Telegram(&tg.sender))
                .await
                <= PERIOD
            {
                return Some(self.standard_queue.remove(index).unwrap());
            }
        }

        None
    }

    pub(crate) async fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(e) => match e {
                    mpsc::error::TryRecvError::Empty => (),
                    mpsc::error::TryRecvError::Disconnected => {
                        tracing::error!("Telegram channel disconnected");
                        break;
                    }
                },
                Ok(command) => {
                    let response = match command.operation {
                        Operation::Queue(params) => {
                            self.queue(params);
                            Response::Success
                        }
                        Operation::Delete(header) => {
                            self.delete(header);
                            Response::Success
                        }
                        Operation::List => Response::List(self.list()),
                    };

                    if let Err(e) = command.tx.send(response) {
                        tracing::error!("Error sending telegram response, {:?}", e);
                    }
                }
            }

            self.send().await;

            tokio::time::sleep(PERIOD).await;
        }
    }
}
