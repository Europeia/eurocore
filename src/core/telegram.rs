use crate::core::error::{ConfigError, Error};
use crate::core::state::AppState;
use crate::ns::telegram::{Telegram, TelegramHeader, TelegramParams, TelegramType};
use crate::types::response;
use crate::utils::ratelimiter::Ratelimiter;
use reqwest::Client;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub(crate) struct Telegrammer {
    client_key: String,
    pub(crate) recruitment_queue: Arc<RwLock<VecDeque<Telegram>>>,
    pub(crate) standard_queue: Arc<RwLock<VecDeque<Telegram>>>,
    ratelimiter: Ratelimiter,
    client: Client,
    url: String,
}

impl Telegrammer {
    pub(crate) fn new(
        user: &str,
        client_key: String,
        ratelimiter: Ratelimiter,
    ) -> Result<Self, ConfigError> {
        let client = Client::builder()
            .user_agent(user)
            .build()
            .map_err(ConfigError::HTTPClient)?;

        let url = "https://www.nationstates.net/cgi-bin/api.cgi".to_string();

        Ok(Self {
            client_key,
            recruitment_queue: Arc::new(RwLock::new(VecDeque::new())),
            standard_queue: Arc::new(RwLock::new(VecDeque::new())),
            ratelimiter,
            client,
            url,
        })
    }

    pub(crate) async fn queue_telegram(&self, params: TelegramParams) {
        match params.telegram_type {
            TelegramType::Recruitment => {
                let telegram = Telegram::from_params(&self.client_key, params);
                self.recruitment_queue.write().await.push_back(telegram);
            }
            TelegramType::Standard => {
                let telegram = Telegram::from_params(&self.client_key, params);
                self.standard_queue.write().await.push_back(telegram);
            }
        }
    }

    pub(crate) async fn delete_telegram(&self, header: TelegramHeader) {
        let mut recruitment_queue = self.recruitment_queue.write().await;

        recruitment_queue.retain(|tg| {
            !(tg.recipient == header.recipient && tg.telegram_id == header.telegram_id)
        });

        let mut standard_queue = self.standard_queue.write().await;

        standard_queue.retain(|tg| {
            !(tg.recipient == header.recipient && tg.telegram_id == header.telegram_id)
        });
    }

    pub(crate) async fn list_telegrams(&self) -> HashMap<String, Vec<response::Telegram>> {
        let mut telegrams: HashMap<String, Vec<response::Telegram>> = HashMap::new();

        let recruitment: Vec<response::Telegram> = self
            .recruitment_queue
            .read()
            .await
            .iter()
            .map(|tg| response::Telegram::new(&tg.recipient, &tg.telegram_id))
            .collect();

        telegrams.insert("recruitment".to_string(), recruitment);

        let standard: Vec<response::Telegram> = self
            .standard_queue
            .read()
            .await
            .iter()
            .map(|tg| response::Telegram::new(&tg.recipient, &tg.telegram_id))
            .collect();

        telegrams.insert("standard".to_string(), standard);

        telegrams
    }

    async fn send(&self) -> Result<(), Error> {
        if self.try_send_recruitment_telegram().await? || self.try_send_standard_telegram().await? {
            Ok(())
        } else {
            tracing::debug!("No telegrams to send");
            Ok(())
        }
    }

    async fn try_send_recruitment_telegram(&self) -> Result<bool, Error> {
        if self.recruitment_queue.read().await.len() > 0 {
            tracing::debug!("Recruitment telegram queue populated");
            if let Some(last_recruitment) = self
                .ratelimiter
                .get_last_recruitment_telegram_timestamp()
                .await
            {
                if last_recruitment.elapsed()
                    > self.ratelimiter.recruitment_telegram_cooldown()
                        - self.ratelimiter.telegram_cooldown()
                {
                    let telegram = self.recruitment_queue.write().await.pop_front().unwrap();
                    self.ratelimiter.acquire_for_recruitment_telegram().await;
                    tracing::debug!("Sending recruitment telegram");

                    self.send_telegram(telegram).await?;

                    Ok(true)
                } else {
                    tracing::debug!("Recruitment cooldown too long, skipping");
                    Ok(false)
                }
            } else {
                let telegram = self.recruitment_queue.write().await.pop_front().unwrap();
                self.ratelimiter.acquire_for_recruitment_telegram().await;
                tracing::debug!("Sending recruitment telegram");

                self.send_telegram(telegram).await?;

                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    async fn try_send_standard_telegram(&self) -> Result<bool, Error> {
        if self.standard_queue.read().await.len() > 0 {
            tracing::debug!("Standard telegram queue populated");
            let telegram = self.standard_queue.write().await.pop_front().unwrap();
            self.ratelimiter.acquire_for_telegram().await;
            tracing::debug!("Sending standard telegram");

            self.send_telegram(telegram).await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn send_telegram(&self, telegram: Telegram) -> Result<(), Error> {
        let _resp = self
            .client
            .get(format!(
                "{}?{}",
                &self.url,
                serde_urlencoded::to_string(telegram)?
            ))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

pub(crate) async fn telegram_loop(state: AppState) {
    loop {
        if let Err(e) = state.client.telegram_queue.send().await {
            tracing::error!("Error sending telegram: {:?}", e);
        };
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
