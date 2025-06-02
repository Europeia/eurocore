use super::PERIOD;
use crate::core::error::{ConfigError, Error};
use crate::ns::rmbpost::{self, Action, Command, IntermediateRmbPost};
use crate::sync::nations;
use crate::sync::ratelimiter;
use crate::types::response::RmbPostStatus;
use crate::utils::encode::encode;
use quick_xml::de;
use regex::Regex;
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;
use std::collections::VecDeque;
use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct Client {
    url: String,
    client: reqwest::Client,
    queue: VecDeque<IntermediateRmbPost>,
    pool: PgPool,
    limiter: ratelimiter::Sender,
    nations: nations::Sender,
    re: Regex,
    rx: mpsc::Receiver<Command>,
}

impl Client {
    fn new(
        user_agent: &str,
        url: &str,
        pool: PgPool,
        limiter: ratelimiter::Sender,
        nations: nations::Sender,
        rx: mpsc::Receiver<Command>,
    ) -> Result<Self, ConfigError> {
        let client = reqwest::Client::builder().user_agent(user_agent).build()?;

        Ok(Self {
            url: url.to_string(),
            client,
            queue: VecDeque::new(),
            pool,
            limiter,
            nations,
            re: Regex::new(r#"=(\d+)#"#)?,
            rx,
        })
    }

    #[tracing::instrument(skip_all)]
    async fn try_post(&mut self) {
        if let Some(post) = self.get_post().await {
            let job_id = post.job_id;

            match self.post(post).await {
                Ok(id) => self.update_job(job_id, "success", Some(id), None).await,
                Err(e) => self.update_job(job_id, "error", None, Some(e)).await,
            }
        }
    }

    #[tracing::instrument(skip_all)]
    async fn get_post(&mut self) -> Option<IntermediateRmbPost> {
        for (index, post) in self.queue.iter().enumerate() {
            if self
                .limiter
                .peek(ratelimiter::Target::Restricted {
                    sender: post.nation.clone(),
                })
                .await
                <= PERIOD
            {
                return Some(self.queue.remove(index).unwrap());
            }
        }

        None
    }

    #[tracing::instrument(skip_all)]
    async fn post(&mut self, mut post: IntermediateRmbPost) -> Result<i32, Error> {
        let password = self.nations.get_password(&post.nation).await?;
        let pin = self
            .nations
            .get_pin(&post.nation)
            .await?
            .unwrap_or_default();

        post.text = encode(&post.text);

        if let Err(duration) = self
            .limiter
            .acquire(ratelimiter::Target::Restricted {
                sender: post.nation.clone(),
            })
            .await
        {
            tokio::time::sleep(duration).await;
        }

        let post = crate::ns::rmbpost::RmbPost::from(post);

        let resp = self
            .client
            .post(&self.url)
            .header("X-Password", &password)
            .header("X-Pin", &pin)
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(serde_urlencoded::to_string(&post)?)
            .send()
            .await?
            .error_for_status()?;

        if let Some(val) = resp.headers().get("X-Pin") {
            self.nations
                .set_pin(post.nation(), val.to_str().map_err(Error::HeaderDecode)?)
                .await?
        }

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if !response.is_ok() {
            return Err(Error::NationStates(response.error.unwrap()));
        }

        let post = post.prepare(response.success.unwrap());

        if let Err(duration) = self.limiter.acquire(ratelimiter::Target::Standard).await {
            tokio::time::sleep(duration).await;
        }

        let resp = self
            .client
            .post(&self.url)
            .header("X-Password", &password)
            .header("X-Pin", &pin)
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(serde_urlencoded::to_string(&post)?)
            .send()
            .await?
            .error_for_status()?;

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if response.is_ok() {
            Ok(self
                .re
                .captures(&response.success.unwrap())
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
                .parse::<i32>()?)
        } else {
            Err(Error::NationStates(response.error.unwrap()))
        }
    }

    #[tracing::instrument(skip_all)]
    async fn process_command(&mut self, command: Command) {
        let response = match command.action {
            Action::Queue { post } => {
                self.queue_post(post).await;
                rmbpost::Response::Success
            }
        };

        if command.tx.send(response).is_err() {
            tracing::error!("failed to send response");
        }
    }

    #[tracing::instrument(skip_all)]
    async fn queue_post(&mut self, post: IntermediateRmbPost) {
        self.queue.push_back(post);
    }

    #[tracing::instrument(skip_all)]
    async fn update_job(
        &self,
        job_id: i32,
        status: &str,
        dispatch_id: Option<i32>,
        error: Option<Error>,
    ) {
        let error: &str = match error {
            Some(err) => &err.to_string(),
            None => "",
        };

        if let Err(e) = sqlx::query(
            "UPDATE rmbpost_queue SET status = $1, rmbpost_id = $2, error = $3, modified_at = $4 WHERE id = $5;",
        )
            .bind(status)
            .bind(dispatch_id)
            .bind(error)
            .bind(chrono::Utc::now())
            .bind(job_id)
            .execute(&self.pool)
            .await
        {
            tracing::error!("{}", e);
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn run(&mut self) {
        let mut interval = tokio::time::interval(PERIOD);

        loop {
            tokio::select! {
                Some(command) = self.rx.recv() => {
                    self.process_command(command).await;
                }

                _ = interval.tick() => {
                    self.try_post().await;
                }
            }
        }
    }
}

fn post_status(row: PgRow) -> RmbPostStatus {
    RmbPostStatus {
        id: row.get("id"),
        status: row.get("status"),
        rmbpost_id: row.get("rmbpost_id"),
        error: row.get("error"),
        created_at: row.get("created_at"),
        modified_at: row.get("modified_at"),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct Response {
    success: Option<String>,
    error: Option<String>,
}

impl Response {
    fn is_ok(&self) -> bool {
        self.success.is_some()
    }
}

pub(crate) fn new(
    user_agent: &str,
    url: &str,
    pool: PgPool,
    limiter: ratelimiter::Sender,
    nations: nations::Sender,
) -> Result<(mpsc::Sender<Command>, Client), ConfigError> {
    let (tx, rx) = mpsc::channel(16);

    let client = Client::new(user_agent, url, pool, limiter, nations, rx)?;

    Ok((tx, client))
}
