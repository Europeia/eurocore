use crate::controllers::dispatch::DispatchController;
use crate::controllers::user::UserController;
use crate::core::client::Client;
use crate::core::error::{ConfigError, Error};
use crate::ns::rmbpost::{IntermediateRmbPost, NewRmbPost};
use crate::ns::telegram;
use crate::ns::{dispatch, rmbpost};
use crate::types::response;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::types::Json;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) client: Client,
    pub(crate) telegram_sender: mpsc::Sender<telegram::Command>,
    pub(crate) dispatch_sender: mpsc::Sender<dispatch::Command>,
    rmbpost_sender: mpsc::Sender<rmbpost::Command>,
    pub(crate) user_controller: UserController,
    pub(crate) dispatch_controller: DispatchController,
}

impl AppState {
    pub(crate) async fn new(
        pool: PgPool,
        secret: String,
        client: Client,
        telegram_sender: mpsc::Sender<telegram::Command>,
        dispatch_sender: mpsc::Sender<dispatch::Command>,
        rmbpost_sender: mpsc::Sender<rmbpost::Command>,
    ) -> Result<Self, ConfigError> {
        Ok(AppState {
            pool: pool.clone(),
            client,
            telegram_sender,
            dispatch_sender,
            rmbpost_sender,
            user_controller: UserController::new(pool.clone(), secret)?,
            dispatch_controller: DispatchController::new(pool),
        })
    }

    pub(crate) async fn queue_rmbpost(
        self,
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

        self.rmbpost_sender
            .send(rmbpost::Command::new(rmbpost, tx))
            .await
            .unwrap();

        if let Err(e) = rx.await {
            tracing::error!("Error sending rmbpost response, {:?}", e);
        }

        Ok(status)
    }

    pub(crate) async fn get_rmbpost_status(
        &self,
        id: i32,
    ) -> Result<response::RmbPostStatus, Error> {
        let status = match sqlx::query(
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
            Ok(status) => status,
            Err(sqlx::Error::RowNotFound) => return Err(Error::JobNotFound),
            Err(e) => return Err(Error::Sql(e)),
        };

        Ok(status)
    }
}

fn map_dispatch_header(row: PgRow) -> response::DispatchHeader {
    response::DispatchHeader {
        id: row.get("dispatch_id"),
        nation: row.get("nation"),
    }
}

fn map_dispatch(row: PgRow) -> response::Dispatch {
    response::Dispatch {
        id: row.get("dispatch_id"),
        nation: row.get("nation"),
        category: row.get("category"),
        subcategory: row.get("subcategory"),
        title: row.get("title"),
        text: row.get("text"),
        created_by: row.get("created_by"),
        modified_at: row.get("created_at"),
    }
}

fn map_dispatch_status(row: PgRow) -> response::DispatchStatus {
    response::DispatchStatus {
        id: row.get("id"),
        action: row.get("action"),
        status: row.get("status"),
        dispatch_id: row.get("dispatch_id"),
        error: row.get("error"),
        created_at: row.get("created_at"),
        modified_at: row.get("modified_at"),
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
