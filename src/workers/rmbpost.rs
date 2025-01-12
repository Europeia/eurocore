use crate::core::client::Client;
use crate::ns::rmbpost::{Command, IntermediateRmbPost, Response};
use crate::utils::ratelimiter::Target;
use sqlx::PgPool;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc::{self, error::TryRecvError};

use super::PERIOD;

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
