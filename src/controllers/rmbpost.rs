use crate::core::error::{ConfigError, Error};
use crate::ns::rmbpost;
use crate::ns::rmbpost::{Action, IntermediateRmbPost, NewRmbPost};
use crate::sync::{nations, ratelimiter};
use crate::types::response;
use crate::workers;
use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug)]
pub(crate) struct Controller {
    pool: PgPool,
    tx: mpsc::Sender<rmbpost::Command>,
}

impl Controller {
    pub(crate) fn new(
        user_agent: &str,
        url: &str,
        pool: PgPool,
        limiter: ratelimiter::Sender,
        nations: nations::Sender,
    ) -> Result<Self, ConfigError> {
        let (tx, mut client) =
            workers::rmbpost::new(user_agent, url, pool.clone(), limiter, nations)?;

        tokio::spawn(async move { client.run().await });

        Ok(Self { pool, tx })
    }

    pub(crate) async fn queue(
        &self,
        rmbpost: NewRmbPost,
    ) -> Result<response::RmbPostStatus, Error> {
        let status = sqlx::query(
            "INSERT INTO rmbpost_queue (nation, region, content, status) VALUES ($1, $2, $3, 'queued') RETURNING
                id,
                status,
                rmbpost_id,
                error,
                created_at,
                modified_at;",
        )
            .bind(&rmbpost.nation)
            .bind(&rmbpost.region)
            .bind(&rmbpost.text)
            .map(map_rmbpost_status)
            .fetch_one(&self.pool)
            .await?;

        let rmbpost =
            IntermediateRmbPost::new(status.id, rmbpost.nation, rmbpost.region, rmbpost.text);

        let (tx, rx) = oneshot::channel();

        self.tx
            .send(rmbpost::Command::new(Action::queue(rmbpost), tx))
            .await
            .unwrap();

        if let Err(e) = rx.await {
            tracing::error!("Error sending rmbpost response, {:?}", e);
        }

        Ok(status)
    }

    pub(crate) async fn get_status(&self, id: i32) -> Result<response::RmbPostStatus, Error> {
        match sqlx::query(
            "SELECT
                id,
                status,
                rmbpost_id,
                error,
                created_at,
                modified_at
            FROM rmbpost_queue
            WHERE id = $1;",
        )
        .bind(id)
        .map(map_rmbpost_status)
        .fetch_one(&self.pool)
        .await
        {
            Ok(status) => Ok(status),
            Err(sqlx::Error::RowNotFound) => return Err(Error::JobNotFound),
            Err(e) => return Err(Error::Sql(e)),
        }
    }
}

fn map_rmbpost_status(row: PgRow) -> response::RmbPostStatus {
    response::RmbPostStatus {
        id: row.get("id"),
        status: row.get("status"),
        rmbpost_id: row.get("rmbpost_id"),
        error: row.get("error"),
        created_at: row.get("created_at"),
        modified_at: row.get("modified_at"),
    }
}
