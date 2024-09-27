use crate::core::error::Error;
use crate::ns::dispatch;
use crate::ns::telegram;
use crate::types::response;
use crate::utils::auth::User;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::Row;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) secret: String,
    pub(crate) telegram_sender: mpsc::Sender<telegram::Command>,
    pub(crate) dispatch_sender: mpsc::Sender<dispatch::Command>,
}

impl AppState {
    pub(crate) async fn new(
        pool: PgPool,
        secret: String,
        telegram_sender: mpsc::Sender<telegram::Command>,
        dispatch_sender: mpsc::Sender<dispatch::Command>,
    ) -> Self {
        AppState {
            pool,
            secret,
            telegram_sender,
            dispatch_sender,
        }
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

    // pub(crate) async fn new_dispatch(
    //     mut self,
    //     params: NewDispatch,
    //     created_by: &str,
    // ) -> Result<response::DispatchHeader, Error> {
    //     if !self.client.contains_nation(&params.nation).await {
    //         return Err(Error::InvalidNation);
    //     }

    //     let command = dispatch::Command::new(Operation::New(params), )

    //     let dispatch = Action::add(params.clone())?;

    //     let dispatch_id = self.client.new_dispatch(dispatch.into()).await?;

    //     let id: i32 = sqlx::query(
    //         "INSERT INTO dispatches (dispatch_id, nation) VALUES ($1, $2) RETURNING id;",
    //     )
    //     .bind(dispatch_id)
    //     .bind(&params.nation)
    //     .map(|row: PgRow| row.get(0))
    //     .fetch_one(&self.pool)
    //     .await?;

    //     sqlx::query(
    //         "INSERT INTO dispatch_content (dispatch_id, category, subcategory, title, text, created_by) VALUES ($1, $2, $3, $4, $5, $6);"
    //     )
    //         .bind(id)
    //         .bind(params.category)
    //         .bind(params.subcategory)
    //         .bind(params.title)
    //         .bind(params.text)
    //         .bind(created_by)
    //         .execute(&self.pool)
    //         .await?;

    //     let dispatch_header = response::DispatchHeader {
    //         id: dispatch_id,
    //         nation: params.nation,
    //     };

    //     Ok(dispatch_header)
    // }

    // pub(crate) async fn edit_dispatch(
    //     mut self,
    //     id: i32,
    //     params: EditDispatch,
    //     created_by: &str,
    // ) -> Result<response::DispatchHeader, Error> {
    //     let nation = self.get_dispatch_nation(id).await?;

    //     let dispatch = Action::edit(id, nation.clone(), params.clone())?;

    //     self.client.new_dispatch(dispatch.into()).await?;

    //     sqlx::query(
    //         "INSERT INTO dispatch_content (dispatch_id, category, subcategory, title, text, created_by) VALUES ((SELECT id FROM dispatches WHERE dispatch_id = $1), $2, $3, $4, $5, $6);",
    //     )
    //         .bind(id)
    //         .bind(params.category)
    //         .bind(params.subcategory)
    //         .bind(params.title)
    //         .bind(params.text)
    //         .bind(created_by)
    //         .execute(&self.pool)
    //         .await?;

    //     let dispatch_header = response::DispatchHeader { id, nation };

    //     Ok(dispatch_header)
    // }

    // pub(crate) async fn remove_dispatch(
    //     mut self,
    //     dispatch_id: i32,
    // ) -> Result<response::DispatchHeader, Error> {
    //     let nation = self.get_dispatch_nation(dispatch_id).await?;

    //     let dispatch = Action::remove(dispatch_id, nation.clone());

    //     self.client.delete_dispatch(dispatch.into()).await?;

    //     sqlx::query("UPDATE dispatches SET is_active = FALSE WHERE dispatch_id = $1;")
    //         .bind(dispatch_id)
    //         .execute(&self.pool)
    //         .await?;

    //     let dispatch_header = response::DispatchHeader {
    //         id: dispatch_id,
    //         nation,
    //     };

    //     Ok(dispatch_header)
    // }

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
                _ => Err(Error::Sql(e)),
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
    }
}
