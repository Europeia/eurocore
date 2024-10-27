use sqlx::postgres::PgPool;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing;

use crate::core::client::Client;
use crate::ns::dispatch::{Action, Command, IntermediateDispatch, Response};
use crate::utils::ratelimiter::Target;
use crate::workers::PERIOD;

#[derive(Debug)]
pub(crate) struct DispatchClient {
    pool: PgPool,
    client: Client,
    queue: VecDeque<IntermediateDispatch>,

    rx: mpsc::Receiver<Command>,
}

impl DispatchClient {
    pub(crate) fn new(pool: PgPool, client: Client, rx: mpsc::Receiver<Command>) -> Self {
        Self {
            pool,
            client,
            rx,
            queue: VecDeque::new(),
        }
    }

    async fn update_job(
        &self,
        job_id: i32,
        status: &str,
        dispatch_id: Option<i32>,
        error: Option<String>,
    ) {
        if let Err(e) = sqlx::query(
            "UPDATE dispatch_queue SET status = $1, dispatch_id = $2, error = $3, modified_at = $4 WHERE id = $5;",
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

    async fn insert_dispatch_header(&self, id: i32, nation: &str) {
        if let Err(e) = sqlx::query("INSERT INTO dispatches (dispatch_id, nation) VALUES ($1, $2);")
            .bind(id)
            .bind(nation)
            .execute(&self.pool)
            .await
        {
            tracing::error!("{}", e);
        }
    }

    async fn insert_dispatch_content(
        &self,
        id: i32,
        category: i16,
        subcategory: i16,
        title: &str,
        text: &str,
        created_by: &str,
    ) {
        if let Err(e) = sqlx::query("INSERT INTO dispatch_content (dispatch_id, category, subcategory, title, text, created_by) VALUES ((SELECT id FROM dispatches WHERE dispatch_id = $1), $2, $3, $4, $5, $6);")
            .bind(id)
            .bind(category)
            .bind(subcategory)
            .bind(title)
            .bind(text)
            .bind(created_by)
            .execute(&self.pool)
            .await
        {
            tracing::error!("{}", e);
        }
    }

    async fn set_dispatch_inactive(&self, id: i32) {
        if let Err(e) = sqlx::query("UPDATE dispatches SET is_active = false WHERE id = $1;")
            .bind(id)
            .execute(&self.pool)
            .await
        {
            tracing::error!("{}", e);
        }
    }

    /// attempts to post a dispatch from the queue
    async fn try_post(&mut self) {
        if let Some(dispatch) = self.try_get_dispatch().await {
            tracing::info!("Eligible dispatch found, posting");
            let job_id = dispatch.job_id;

            let (status, dispatch_id, error) =
                match self.client.post_dispatch(dispatch.clone()).await {
                    Ok(id) => ("success", Some(id), None),
                    Err(e) => ("failure", None, Some(e.to_string())),
                };

            self.update_job(job_id, status, dispatch_id, error).await;

            if let Some(id) = dispatch_id {
                match dispatch.action {
                    Action::Add {
                        title,
                        text,
                        category,
                    } => {
                        self.insert_dispatch_header(id, &dispatch.nation).await;

                        let (category, subcategory) = category.to_tuple();

                        self.insert_dispatch_content(
                            id,
                            category,
                            subcategory,
                            &title,
                            &text,
                            &dispatch.user,
                        )
                        .await;
                    }
                    Action::Edit {
                        id,
                        title,
                        text,
                        category,
                    } => {
                        let (category, subcategory) = category.to_tuple();

                        self.insert_dispatch_content(
                            id,
                            category,
                            subcategory,
                            &title,
                            &text,
                            &dispatch.user,
                        )
                        .await;
                    }
                    Action::Remove { id } => {
                        self.set_dispatch_inactive(id).await;
                    }
                }
            }
        }
    }

    /// checks queue for dispatch that is eligible to be posted within the current `PERIOD`
    async fn try_get_dispatch(&mut self) -> Option<IntermediateDispatch> {
        for (index, dispatch) in self.queue.iter().enumerate() {
            if self.post_in(dispatch).await <= PERIOD {
                return Some(self.queue.remove(index).unwrap());
            }
        }

        None
    }

    async fn post_in(&self, dispatch: &IntermediateDispatch) -> Duration {
        match dispatch.action {
            Action::Add { .. } => {
                self.client
                    .ratelimiter
                    .peek_ratelimit(Target::Restricted(&dispatch.nation))
                    .await
            }
            Action::Edit { .. } | Action::Remove { .. } => {
                self.client
                    .ratelimiter
                    .peek_ratelimit(Target::Standard)
                    .await
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(e) => match e {
                    mpsc::error::TryRecvError::Empty => (),
                    mpsc::error::TryRecvError::Disconnected => {
                        tracing::error!("Dispatch channel disconnected");
                        break;
                    }
                },
                Ok(command) => {
                    tracing::info!("Queueing job");
                    self.queue.push_back(command.dispatch);

                    if let Err(e) = command.tx.send(Response::Success) {
                        tracing::error!("Error sending dispatch response, {:?}", e);
                    }
                }
            }

            self.try_post().await;

            tokio::time::sleep(PERIOD).await;
        }
    }
}
