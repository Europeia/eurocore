use crate::controllers::user::UserController;
use crate::core::client::Client;
use crate::core::error::{ConfigError, Error};
use crate::ns::rmbpost::{IntermediateRmbPost, NewRmbPost};
use crate::ns::telegram;
use crate::ns::{dispatch, rmbpost};
use crate::types::response;
use crate::types::{AuthorizedUser, Username};
use serde::Serialize;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::types::Json;
use sqlx::Row;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) secret: String,
    pub(crate) client: Client,
    pub(crate) telegram_sender: mpsc::Sender<telegram::Command>,
    pub(crate) dispatch_sender: mpsc::Sender<dispatch::Command>,
    rmbpost_sender: mpsc::Sender<rmbpost::Command>,
    username_re: regex::Regex,
    pub(crate) user_controller: UserController,
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
            pool,
            secret,
            client,
            telegram_sender,
            dispatch_sender,
            rmbpost_sender,
            username_re: regex::Regex::new(r"^[a-zA-Z0-9_-]{3,20}$")?,
        })
    }

    pub(crate) async fn queue_dispatch<T: Serialize>(
        &self,
        action: &str,
        payload: Json<T>,
    ) -> Result<response::DispatchStatus, Error> {
        let status = sqlx::query(
            "INSERT INTO dispatch_queue (type, payload, status) VALUES ($1, $2, 'queued')
            RETURNING
                id,
                type AS action,
                status,
                dispatch_id,
                error,
                created_at,
                modified_at;",
        )
        .bind(action)
        .bind(payload)
        .map(map_dispatch_status)
        .fetch_one(&self.pool)
        .await?;

        Ok(status)
    }

    pub(crate) async fn get_dispatch_status(
        &self,
        id: i32,
    ) -> Result<response::DispatchStatus, Error> {
        let status = match sqlx::query(
            "SELECT
                id,
                type AS action,
                status,
                dispatch_id,
                error,
                created_at,
                modified_at
            FROM dispatch_queue
            WHERE id = $1;",
        )
        .bind(id)
        .map(map_dispatch_status)
        .fetch_one(&self.pool)
        .await
        {
            Ok(status) => status,
            Err(sqlx::Error::RowNotFound) => return Err(Error::JobNotFound),
            Err(e) => return Err(Error::Sql(e)),
        };

        Ok(status)
    }

    pub(crate) async fn get_dispatch_nation(&self, dispatch_id: i32) -> Result<String, Error> {
        let nation: String = match sqlx::query(
            "SELECT nation FROM dispatches WHERE dispatch_id = $1 AND is_active = TRUE;",
        )
        .bind(dispatch_id)
        .map(|row: PgRow| row.get("nation"))
        .fetch_one(&self.pool)
        .await
        {
            Ok(nation) => nation,
            Err(sqlx::Error::RowNotFound) => return Err(Error::DispatchNotFound),
            Err(e) => return Err(Error::Sql(e)),
        };

        Ok(nation)
    }

    pub(crate) async fn get_dispatch(self, dispatch_id: i32) -> Result<response::Dispatch, Error> {
        let dispatch = match sqlx::query(
            "SELECT
                dispatches.dispatch_id,
                dispatches.nation,
                dispatch_content.category,
                dispatch_content.subcategory,
                dispatch_content.title,
                dispatch_content.text,
                dispatch_content.created_by,
                dispatch_content.created_at as created_at
            FROM dispatches
            JOIN
                dispatch_content ON dispatch_content.dispatch_id = dispatches.id
            WHERE dispatches.dispatch_id = $1
            AND dispatches.is_active = TRUE;",
        )
        .bind(dispatch_id)
        .map(map_dispatch)
        .fetch_one(&self.pool)
        .await
        {
            Ok(dispatch) => dispatch,
            Err(sqlx::Error::RowNotFound) => return Err(Error::DispatchNotFound),
            Err(e) => return Err(Error::Sql(e)),
        };

        Ok(dispatch)
    }

    async fn get_all_dispatches(&self) -> Result<Vec<response::Dispatch>, Error> {
        let dispatches = sqlx::query(
            "SELECT
                dispatches.dispatch_id,
                dispatches.nation,
                dispatch_content.category,
                dispatch_content.subcategory,
                dispatch_content.title,
                dispatch_content.text,
                dispatch_content.created_by,
                dispatch_content.created_at as created_at
            FROM dispatches
            JOIN
                dispatch_content ON dispatch_content.dispatch_id = dispatches.id
            WHERE dispatch_content.id = (
                SELECT id FROM dispatch_content
              WHERE dispatch_content.dispatch_id = dispatches.id
              ORDER BY dispatch_content.id DESC
              LIMIT 1
            )
            AND dispatches.is_active = TRUE;",
        )
        .map(map_dispatch)
        .fetch_all(&self.pool)
        .await?;

        Ok(dispatches)
    }

    async fn get_dispatches_by_nation(
        &self,
        nation: String,
    ) -> Result<Vec<response::Dispatch>, Error> {
        let dispatches = sqlx::query(
            "SELECT
                dispatches.dispatch_id,
                dispatches.nation,
                dispatch_content.category,
                dispatch_content.subcategory,
                dispatch_content.title,
                dispatch_content.text,
                dispatch_content.created_by,
                dispatch_content.created_at as created_at
            FROM dispatches
            JOIN
                dispatch_content ON dispatch_content.dispatch_id = dispatches.id
            WHERE dispatch_content.id = (
                SELECT id FROM dispatch_content
              WHERE dispatch_content.dispatch_id = dispatches.id
              ORDER BY dispatch_content.id DESC
              LIMIT 1
            )
            AND dispatches.is_active = TRUE
            AND dispatches.nation = $1;",
        )
        .bind(nation)
        .map(map_dispatch)
        .fetch_all(&self.pool)
        .await?;

        Ok(dispatches)
    }

    pub(crate) async fn get_dispatches(
        self,
        nation: Option<String>,
    ) -> Result<Vec<response::Dispatch>, Error> {
        let dispatches = match nation {
            Some(nation) => self.get_dispatches_by_nation(nation).await?,
            None => self.get_all_dispatches().await?,
        };

        Ok(dispatches)
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

    fn hash(&self, value: &str) -> Result<String, Error> {
        bcrypt::hash(value, 12).map_err(Error::Bcrypt)
    }

    pub(crate) async fn update_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(), Error> {
        sqlx::query("UPDATE users SET password_hash = $1 WHERE username = $2;")
            .bind(self.hash(password)?)
            .bind(username)
            .execute(&self.pool)
            .await?;

        Ok(())
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
