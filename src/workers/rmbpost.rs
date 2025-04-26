use super::PERIOD;
// use crate::core::client::Client;
use crate::core::error::{ConfigError, Error};
use crate::ns::rmbpost::{self, Command, IntermediateRmbPost, NewRmbPost};
use crate::sync::ratelimiter;
use crate::types::response;
use crate::types::response::RmbPostStatus;
use crate::utils::encode::encode;
use crate::utils::ratelimiter::Target;
use quick_xml::de;
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::postgres::PgRow;
use std::collections::VecDeque;
use tokio::sync::mpsc::{self, error::TryRecvError};
use tokio::time::Duration;

const MAX_COOLDOWN: Duration = Duration::from_millis(100);

#[derive(Debug)]
struct Client {
    url: String,
    client: reqwest::Client,
    queue: VecDeque<IntermediateRmbPost>,
    pool: PgPool,
    limiter: ratelimiter::Sender,
    rx: mpsc::Receiver<Command>,
}

impl Client {
    fn new(
        user_agent: &str,
        url: &str,
        pool: PgPool,
        limiter: ratelimiter::Sender,
        rx: mpsc::Receiver<Command>,
    ) -> Result<Self, ConfigError> {
        let client = reqwest::Client::builder().user_agent(user_agent).build()?;

        Ok(Self {
            url: url.to_string(),
            client,
            queue: VecDeque::new(),
            pool,
            limiter,
            rx,
        })
    }

    async fn try_post(&mut self) {
        if let Some(post) = self.get_post().await {
            if let Err(e) = self.post(post).await {
                tracing::error!("failed to publish post: {}", e);
            }
        }
    }

    async fn get_post(&mut self) -> Option<IntermediateRmbPost> {
        for (index, post) in self.queue.iter().enumerate() {
            if self
                .limiter
                .peek(ratelimiter::Target::Restricted {
                    sender: post.nation.clone(),
                })
                .await
                <= MAX_COOLDOWN
            {
                return Some(self.queue.remove(index).unwrap());
            }
        }

        None
    }

    async fn post(&mut self, mut post: IntermediateRmbPost) -> Result<(), Error> {
        let password: String = self.nations.get_password(&post.nation).await?;
        let pin: Option<String> = self.nations.get_pin(&post.nation).await?;

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
            .header("X-Password", password)
            .header("X-Pin", pin.unwrap_or_default())
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(serde_urlencoded::to_string(&post)?)
            .send()
            .await?
            .error_for_status()?;

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if let Some(val) = resp.headers().get("X-Pin") {
            self.nations
                .set_pin(&post.nation, val.to_str().map_err(Error::HeaderDecode)?)
        }

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
            .header("X-Password", password)
            .header("X-Pin", pin.unwrap_or_default())
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(serde_urlencoded::to_string(&post)?)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn process_command(&mut self, command: Command) {
        let response = match command.action {
            Action::Queue { post } => {
                if let Err(e) = self.queue_post(post).await {
                    Response::Error(e)
                } else {
                    Response::Success
                }
            }
        };

        if let Err(_) = command.tx.send(response) {
            tracing::error!("failed to send response");
        }
    }

    async fn queue_post(&mut self, post: NewRmbPost) -> Result<RmbPostStatus, Error> {
        let status = sqlx::query(
            "INSERT INTO
          rmbpost_queue (nation, region, content, status)
        VALUES
          ($1, $2, $3, 'queued')
        RETURNING
          id,
          status,
          rmbpost_id,
          error,
          created_at,
          modified_at;",
        )
        .bind(&post.nation)
        .bind(&post.region)
        .bind(&post.text)
        .map(post_status)
        .fetch_one(&self.pool)
        .await?;

        self.queue.push_back(IntermediateRmbPost::new(
            status.id,
            post.nation,
            post.region,
            post.text,
        ));

        Ok(status)
    }

    async fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!("client disconnected");
                    break;
                }
                Ok(command) => self.process_command(command).await,
            }

            tokio::time::sleep(MAX_COOLDOWN).await;
        }
    }
}

#[derive(Debug)]
pub(crate) struct RmbPostClient {
    pool: PgPool,
    client: Client,
    queue: VecDeque<IntermediateRmbPost>,

    rx: mpsc::Receiver<Command>,
}

impl RmbPostClient {
    pub(crate) fn new(pool: PgPool, client: Client, rx: mpsc::Receiver<Command>) -> Self {
        Self {
            pool,
            client,
            rx,
            queue: VecDeque::new(),
        }
    }

    async fn post_in(&self, rmbpost: &IntermediateRmbPost) -> Duration {
        self.client
            .ratelimiter
            .peek_ratelimit(Target::Restricted(&rmbpost.nation))
            .await
    }

    async fn try_get_rmbpost(&mut self) -> Option<IntermediateRmbPost> {
        for (index, rmbpost) in self.queue.iter().enumerate() {
            if self.post_in(rmbpost).await <= PERIOD {
                return Some(self.queue.remove(index).unwrap());
            }
        }

        None
    }

    async fn update_job(
        &self,
        job_id: i32,
        status: &str,
        dispatch_id: Option<i32>,
        error: Option<String>,
    ) {
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

    async fn try_post(&mut self) {
        if let Some(rmbpost) = self.try_get_rmbpost().await {
            tracing::info!("Eligible rmbpost found, posting");

            let job_id = rmbpost.job_id;

            let (status, rmbpost_id, error) = match self.client.post_rmbpost(rmbpost).await {
                Ok(rmbpost_id) => ("success", Some(rmbpost_id), None),
                Err(e) => {
                    tracing::error!("Error posting rmbpost, {:?}", e);
                    ("error", None, Some(e.to_string()))
                }
            };

            self.update_job(job_id, status, rmbpost_id, error).await;
        }
    }

    pub(crate) async fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(e) => match e {
                    TryRecvError::Empty => (),
                    TryRecvError::Disconnected => {
                        tracing::error!("Rmbpost channel disconnected");
                        break;
                    }
                },
                Ok(command) => {
                    tracing::info!("Queueing job");
                    self.queue.push_back(command.rmbpost);

                    if let Err(e) = command.tx.send(Response::Success) {
                        tracing::error!("Error sending rmbpost response, {:?}", e);
                    }
                }
            }

            self.try_post().await;

            tokio::time::sleep(PERIOD).await;
        }
    }
}

fn post_status(row: PgRow) -> response::RmbPostStatus {
    response::RmbPostStatus {
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
