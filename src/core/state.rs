use crate::core::client::Client;
use crate::core::error::{ConfigError, Error};
use crate::types::ns::{Dispatch, EditDispatchParams, NewDispatchParams};
use crate::utils::auth::User;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) client: Client,
}

impl AppState {
    pub(crate) async fn new(
        database_url: &str,
        user: &str,
        nation: String,
        password: String,
    ) -> Result<Self, ConfigError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(ConfigError::DatabaseConnectionFailure)?;

        let client = Client::new(user, nation, password)?;

        Ok(AppState { pool, client })
    }

    pub(crate) async fn new_dispatch(mut self, params: NewDispatchParams) -> Result<i32, Error> {
        let dispatch = Dispatch::try_from_new_params(params, &self.client.nation)?;

        let dispatch_id = self.client.new_dispatch(dispatch.clone()).await?;

        let id: i32 = sqlx::query("INSERT INTO dispatches (dispatch_id) VALUES ($1) RETURNING id;")
            .bind(dispatch_id)
            .map(|row: PgRow| row.get(0))
            .fetch_one(&self.pool)
            .await
            .map_err(Error::SQL)?;

        sqlx::query(
            "INSERT INTO dispatch_content (dispatch_id, category, subcategory, title, text, created_by) VALUES ($1, $2, $3, $4, $5, $6);"
        )
            .bind(id)
            .bind(dispatch.category)
            .bind(dispatch.subcategory)
            .bind(dispatch.title)
            .bind(dispatch.text)
            .bind("upc")
            .execute(&self.pool)
            .await
            .map_err(Error::SQL)?;

        Ok(dispatch_id)
    }

    pub(crate) async fn edit_dispatch(mut self, params: EditDispatchParams) -> Result<i32, Error> {
        let dispatch = Dispatch::try_from_edit_params(params, &self.client.nation)?;

        let dispatch_id: i32 = match sqlx::query(
            "SELECT dispatch_id FROM dispatches WhERE is_active = true AND dispatch_id = $1;",
        )
        .bind(dispatch.id.unwrap())
        .map(|row: PgRow| row.get(0))
        .fetch_one(&self.pool)
        .await
        {
            Ok(id) => id,
            Err(sqlx::Error::RowNotFound) => return Err(Error::DispatchNotFound),
            Err(e) => return Err(Error::SQL(e)),
        };

        self.client.new_dispatch(dispatch.clone()).await?;

        sqlx::query(
            "INSERT INTO dispatch_content (dispatch_id, category, subcategory, title, text, created_by) VALUES ((SELECT id FROM dispatches WHERE dispatch_id = $1), $2, $3, $4, $5, $6);",
        )
            .bind(dispatch_id)
            .bind(dispatch.category)
            .bind(dispatch.subcategory)
            .bind(dispatch.title)
            .bind(dispatch.text)
            .bind("upc")
            .execute(&self.pool)
            .await
            .map_err(Error::SQL)?;

        Ok(dispatch_id)
    }

    pub(crate) async fn retrieve_user_by_nation(
        &self,
        nation: &str,
    ) -> Result<Option<User>, Error> {
        match sqlx::query("SELECT nation, password_hash FROM users WHERE nation = $1;")
            .bind(nation)
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
        nation: row.get(1),
        password_hash: row.get(2),
    }
}
