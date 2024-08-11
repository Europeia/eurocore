use crate::core::client::Client;
use crate::core::error::{ConfigError, Error};
use crate::types::ns::{Dispatch, EditDispatchParams, NewDispatchParams, RemoveDispatchParams};
use crate::types::response;
use crate::utils::auth::User;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) client: Client,
    pub(crate) secret: String,
}

impl AppState {
    pub(crate) async fn new(
        database_url: &str,
        user: &str,
        nations: HashMap<String, String>,
        secret: String,
        telegram_client_key: String,
    ) -> Result<Self, ConfigError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let client = Client::new(user, nations, telegram_client_key)?;

        Ok(AppState {
            pool,
            client,
            secret,
        })
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
                dispatch_content.created_by
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
            Err(e) => return Err(Error::SQL(e)),
        };

        Ok(dispatch)
    }

    pub(crate) async fn get_dispatches(self) -> Result<Vec<response::DispatchHeader>, Error> {
        let dispatches = sqlx::query(
            "SELECT
                dispatches.dispatch_id,
                dispatches.nation,
                dispatch_content.category,
                dispatch_content.subcategory,
                dispatch_content.title,
                dispatch_content.created_by
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
        .map(map_dispatch_header)
        .fetch_all(&self.pool)
        .await?;

        Ok(dispatches)
    }

    pub(crate) async fn new_dispatch(
        mut self,
        params: NewDispatchParams,
        created_by: &str,
    ) -> Result<response::DispatchHeader, Error> {
        if !self.client.nations.contains_key(&params.nation) {
            return Err(Error::InvalidNation);
        }

        let dispatch = Dispatch::try_from(params.clone())?;

        let dispatch_id = self.client.new_dispatch(dispatch.clone()).await?;

        let id: i32 = sqlx::query(
            "INSERT INTO dispatches (dispatch_id, nation) VALUES ($1, $2) RETURNING id;",
        )
        .bind(dispatch_id)
        .bind(dispatch.nation)
        .map(|row: PgRow| row.get(0))
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO dispatch_content (dispatch_id, category, subcategory, title, text, created_by) VALUES ($1, $2, $3, $4, $5, $6);"
        )
            .bind(id)
            .bind(dispatch.category)
            .bind(dispatch.subcategory)
            .bind(dispatch.title)
            .bind(dispatch.text)
            .bind(created_by)
            .execute(&self.pool)
            .await?;

        let dispatch_header = response::DispatchHeader {
            id: dispatch_id,
            nation: params.nation,
            category: Some(params.category),
            subcategory: Some(params.subcategory),
            title: Some(params.title),
            created_by: Some(created_by.to_string()),
        };

        Ok(dispatch_header)
    }

    pub(crate) async fn edit_dispatch(
        mut self,
        params: EditDispatchParams,
        created_by: &str,
    ) -> Result<response::DispatchHeader, Error> {
        if !self.client.nations.contains_key(&params.nation) {
            return Err(Error::InvalidNation);
        }

        let dispatch = Dispatch::try_from(params.clone())?;

        let dispatch_id = self.get_dispatch_id(&dispatch).await?;

        self.client.new_dispatch(dispatch.clone()).await?;

        sqlx::query(
            "INSERT INTO dispatch_content (dispatch_id, category, subcategory, title, text, created_by) VALUES ((SELECT id FROM dispatches WHERE dispatch_id = $1), $2, $3, $4, $5, $6);",
        )
            .bind(dispatch_id)
            .bind(dispatch.category)
            .bind(dispatch.subcategory)
            .bind(dispatch.title)
            .bind(dispatch.text)
            .bind(created_by)
            .execute(&self.pool)
            .await?;

        let dispatch_header = response::DispatchHeader {
            id: dispatch_id,
            nation: params.nation,
            category: Some(params.category),
            subcategory: Some(params.subcategory),
            title: Some(params.title),
            created_by: Some(created_by.to_string()),
        };

        Ok(dispatch_header)
    }

    pub(crate) async fn remove_dispatch(
        mut self,
        params: RemoveDispatchParams,
    ) -> Result<response::DispatchHeader, Error> {
        if !self.client.nations.contains_key(&params.nation) {
            return Err(Error::InvalidNation);
        }

        let dispatch = Dispatch::try_from(params.clone())?;

        let dispatch_id = self.get_dispatch_id(&dispatch).await?;

        self.client.delete_dispatch(dispatch.clone()).await?;

        sqlx::query("UPDATE dispatches SET is_active = FALSE WHERE dispatch_id = $1;")
            .bind(dispatch_id)
            .execute(&self.pool)
            .await?;

        let dispatch_header = response::DispatchHeader {
            id: dispatch_id,
            nation: params.nation,
            ..Default::default()
        };

        Ok(dispatch_header)
    }

    async fn get_dispatch_id(&self, dispatch: &Dispatch) -> Result<i32, Error> {
        Ok(
            match sqlx::query(
                "SELECT dispatch_id FROM dispatches WhERE is_active = true AND dispatch_id = $1;",
            )
            .bind(dispatch.id.unwrap())
            .map(|row: PgRow| row.get("dispatch_id"))
            .fetch_one(&self.pool)
            .await
            {
                Ok(id) => id,
                Err(sqlx::Error::RowNotFound) => return Err(Error::DispatchNotFound),
                Err(e) => return Err(Error::SQL(e)),
            },
        )
    }

    pub(crate) async fn register_user(
        &self,
        nation: &str,
        password_hash: &str,
    ) -> Result<User, Error> {
        if let Err(e) = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2);")
            .bind(nation)
            .bind(password_hash)
            .execute(&self.pool)
            .await
        {
            return match e {
                sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                    Err(Error::UserAlreadyExists)
                }
                _ => Err(Error::SQL(e)),
            };
        }

        Ok(User {
            username: nation.to_string(),
            password_hash: password_hash.to_string(),
            claims: sqlx::types::Json(Vec::new()),
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
            json_agg(permissions.name) AS permissions
            FROM
                users
            JOIN
                user_permissions ON users.id = user_permissions.user_id
            JOIN
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
            Err(e) => Err(Error::SQL(e)),
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
            json_agg(permissions.name) AS permissions
            FROM
                api_keys
            JOIN
                users ON api_keys.user_id = users.id
            JOIN
                user_permissions ON users.id = user_permissions.user_id
            JOIN
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
            Err(e) => Err(Error::SQL(e)),
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
        category: row.get("category"),
        subcategory: row.get("subcategory"),
        title: row.get("title"),
        created_by: row.get("created_by"),
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
    }
}
