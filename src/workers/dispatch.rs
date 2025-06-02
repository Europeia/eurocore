use super::PERIOD;
use crate::core::error::{ConfigError, Error};
use crate::ns::dispatch::{self, Action, Command, Dispatch, IntermediateDispatch};
use crate::ns::types::Mode;
use crate::sync::{
    nations,
    ratelimiter::{self, Target},
};
use crate::utils::encode::encode;
use quick_xml::de;
use regex::Regex;
use serde::Deserialize;
use sqlx::postgres::PgPool;
use std::collections::VecDeque;
use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct Client {
    url: String,
    client: reqwest::Client,
    pool: PgPool,
    queue: VecDeque<IntermediateDispatch>,
    limiter: ratelimiter::Sender,
    nations: nations::Sender,
    rx: mpsc::Receiver<Command>,
    re: Regex,
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
            pool,
            queue: VecDeque::new(),
            limiter,
            nations,
            rx,
            re: Regex::new(r#"(\d+)"#)?,
        })
    }

    #[tracing::instrument(skip_all)]
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

    #[tracing::instrument(skip_all)]
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

    #[tracing::instrument(skip_all)]
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

    #[tracing::instrument(skip_all)]
    async fn set_dispatch_inactive(&self, id: i32) {
        if let Err(e) = sqlx::query("UPDATE dispatches SET is_active = false WHERE id = $1;")
            .bind(id)
            .execute(&self.pool)
            .await
        {
            tracing::error!("{}", e);
        }
    }

    #[tracing::instrument(skip_all)]
    async fn post(&mut self, mut dispatch: IntermediateDispatch) -> Result<i32, Error> {
        tracing::debug!("getting nation password");
        let password = self.nations.get_password(&dispatch.nation).await?;

        let dispatch_id = match dispatch.action {
            Action::Add { .. } => None,
            Action::Edit { id, .. } => Some(id),
            Action::Remove { id } => Some(id),
        };

        let acquire = match &mut dispatch.action {
            Action::Add { text, .. } => {
                *text = encode(text);

                self.limiter
                    .acquire(Target::restricted(&dispatch.nation))
                    .await
            }
            Action::Edit { text, .. } => {
                *text = encode(text);

                self.limiter.acquire(Target::Standard).await
            }
            Action::Remove { .. } => self.limiter.acquire(Target::Standard).await,
        };

        if let Err(duration) = acquire {
            tracing::info!("sleeping for {}ms", duration.as_millis());
            tokio::time::sleep(duration).await;
        };

        let mut dispatch = Dispatch::from(dispatch);

        tracing::debug!("getting pin");
        let pin = self
            .nations
            .get_pin(&dispatch.nation)
            .await?
            .unwrap_or_default();

        tracing::debug!("executing prepare request");
        let resp = self
            .client
            .post(&self.url)
            .header("X-Password", &password)
            .header("X-Pin", pin)
            .body(serde_urlencoded::to_string(&dispatch)?)
            .send()
            .await?
            .error_for_status()?;

        if let Some(val) = resp.headers().get("X-Pin") {
            self.nations
                .set_pin(&dispatch.nation, val.to_str()?)
                .await?;
        };

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if !response.is_ok() {
            return Err(Error::NationStates(response.error.unwrap()));
        }

        dispatch.set_mode(Mode::Execute);
        dispatch.set_token(response.success.unwrap());

        if let Err(duration) = self.limiter.acquire(Target::Standard).await {
            tracing::info!("sleeping for {}ms", duration.as_millis());
            tokio::time::sleep(duration).await;
        };

        tracing::debug!("executing execute request");
        let resp = self
            .client
            .post(&self.url)
            .header("X-Password", &password)
            .header(
                "X-Pin",
                self.nations
                    .get_pin(&dispatch.nation)
                    .await?
                    .unwrap_or_default(),
            )
            .body(serde_urlencoded::to_string(&dispatch)?)
            .send()
            .await?
            .error_for_status()?;

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if response.is_ok() {
            // is this a stupid way to do this? idk, maybe
            // but also, the only instance where dispatch_id will be None is for a new dispatch
            // in which case, the response returned from NS 100% contains the id for the new dispatch
            // it would be so much cooler if we could always reply on the response containing the id
            // but alas
            match dispatch_id {
                Some(id) => Ok(id),
                None => Ok(self
                    .re
                    .find(&response.success.unwrap())
                    .unwrap()
                    .as_str()
                    .parse()?),
            }
        } else {
            Err(Error::NationStates(response.error.unwrap()))
        }
    }

    #[tracing::instrument(skip_all)]
    async fn get_dispatch(&mut self) -> Option<IntermediateDispatch> {
        for (index, dispatch) in self.queue.iter().enumerate() {
            if self
                .limiter
                .peek(Target::restricted(&dispatch.nation))
                .await
                <= PERIOD
            {
                tracing::info!("eligible dispatch found");
                return Some(self.queue.remove(index).unwrap());
            }
        }

        None
    }

    #[tracing::instrument(skip_all)]
    async fn try_post(&mut self) {
        if let Some(dispatch) = self.get_dispatch().await {
            let job_id = dispatch.job_id;
            tracing::debug!("job id: {}", job_id);

            let (status, dispatch_id, error) = match self.post(dispatch.clone()).await {
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

    #[tracing::instrument(skip_all)]
    async fn process_command(&mut self, command: Command) {
        tracing::info!("received command");
        self.queue.push_back(command.dispatch);

        if command.tx.send(dispatch::Response::Success).is_err() {
            tracing::error!("failed to send response");
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

                _  = interval.tick() => {
                    self.try_post().await;
                }
            }
        }
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
    user: &str,
    url: &str,
    pool: PgPool,
    limiter: ratelimiter::Sender,
    nations: nations::Sender,
) -> Result<(mpsc::Sender<Command>, Client), ConfigError> {
    let (tx, rx) = mpsc::channel(16);

    let client = Client::new(user, url, pool, limiter, nations, rx)?;

    Ok((tx, client))
}
