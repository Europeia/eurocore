use serde::Serialize;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::types::Json;
use sqlx::Row;
use tokio::sync::mpsc;

use crate::core::client::Client;
use crate::core::error::{ConfigError, Error};
use crate::ns::dispatch;
use crate::ns::telegram;
use crate::types::response;
use crate::utils::auth::User;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) secret: String,
    pub(crate) client: Client,
    pub(crate) telegram_sender: mpsc::Sender<telegram::Command>,
    pub(crate) dispatch_sender: mpsc::Sender<dispatch::Command>,
    username_re: regex::Regex,
}

impl AppState {
    pub(crate) async fn new(
        pool: PgPool,
        secret: String,
        client: Client,
        telegram_sender: mpsc::Sender<telegram::Command>,
        dispatch_sender: mpsc::Sender<dispatch::Command>,
    ) -> Result<Self, ConfigError> {
        Ok(AppState {
            pool,
            secret,
            client,
            telegram_sender,
            dispatch_sender,
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
                timezone('utc', created_at) as created_at,
                timezone('utc', modified_at) as modified_at;",
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
                timezone('utc', created_at) as created_at,
                timezone('utc', modified_at) as modified_at
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
                timezone('utc', dispatch_content.created_at) as created_at
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
                timezone('utc', dispatch_content.created_at) as created_at
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
                timezone('utc', dispatch_content.created_at) as created_at
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

    pub(crate) async fn register_user(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<User, Error> {
        if !self.username_re.is_match(username) {
            return Err(Error::InvalidUsername);
        }

        if let Err(e) = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2);")
            .bind(username)
            .bind(password_hash)
            .execute(&self.pool)
            .await
        {
            return match e {
                sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                    Err(Error::UserAlreadyExists)
                }
                _ => Err(Error::Sql(e)),
            };
        }

        Ok(User {
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            claims: Vec::new(),
        })
    }

    pub(crate) async fn retrieve_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<User>, Error> {
        match sqlx::query(
            "SELECT
            users.username,
            users.password_hash,
            COALESCE(array_agg(permissions.name), '{}') AS permissions
            FROM
                users
            LEFT JOIN
                user_permissions ON users.id = user_permissions.user_id
            LEFT JOIN
                permissions ON user_permissions.permission_id = permissions.id
            WHERE
                users.username = $1
            GROUP BY
                users.id, users.username;",
        )
        .bind(username)
        .map(map_user)
        .fetch_one(&self.pool)
        .await
        {
            Ok(user) => Ok(Some(user)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(Error::Sql(e)),
        }
    }

    pub(crate) async fn retrieve_user_by_api_key(
        &self,
        api_key: &str,
    ) -> Result<Option<User>, Error> {
        match sqlx::query(
            "SELECT
            users.username,
            users.password_hash,
            COALESCE(array_agg(permissions.name), '{}') AS permissions
            FROM
                api_keys
            JOIN
                users ON api_keys.user_id = users.id
            LEFT JOIN
                user_permissions ON users.id = user_permissions.user_id
            LEFT JOIN
                permissions ON user_permissions.permission_id = permissions.id
            WHERE
                key = $1
            GROUP BY
                users.id, users.username;",
        )
        .bind(api_key)
        .map(map_user)
        .fetch_one(&self.pool)
        .await
        {
            Ok(user) => Ok(Some(user)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(Error::Sql(e)),
        }
    }
}

fn map_user(row: PgRow) -> User {
    User {
        username: row.get("username"),
        password_hash: row.get("password_hash"),
        claims: row.get("permissions"),
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
