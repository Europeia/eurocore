use crate::core::error::Error;
use crate::types::response;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

#[derive(Clone, Debug)]
pub(crate) struct DispatchController {
    pool: PgPool,
}

impl DispatchController {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // TODO: refactor this to take a more specific type than anything that implements Serialize
    pub(crate) async fn queue<T: Serialize>(
        &self,
        action: &str,
        payload: Json<T>,
    ) -> Result<response::DispatchStatus, Error> {
        Ok(sqlx::query(
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
        .await?)
    }

    pub(crate) async fn get_status(&self, id: i32) -> Result<response::DispatchStatus, Error> {
        match sqlx::query(
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
            Ok(status) => Ok(status),
            Err(sqlx::Error::RowNotFound) => return Err(Error::JobNotFound),
            Err(e) => return Err(Error::Sql(e)),
        }
    }

    pub(crate) async fn get_nation(&self, dispatch_id: i32) -> Result<String, Error> {
        match sqlx::query(
            "SELECT nation FROM dispatches WHERE dispatch_id = $1 AND is_active = TRUE;",
        )
        .bind(dispatch_id)
        .map(|row: PgRow| row.get("nation"))
        .fetch_one(&self.pool)
        .await
        {
            Ok(nation) => Ok(nation),
            Err(sqlx::Error::RowNotFound) => return Err(Error::DispatchNotFound),
            Err(e) => return Err(Error::Sql(e)),
        }
    }

    pub(crate) async fn get_one(self, dispatch_id: i32) -> Result<response::Dispatch, Error> {
        match sqlx::query(
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
            Ok(dispatch) => Ok(dispatch),
            Err(sqlx::Error::RowNotFound) => return Err(Error::DispatchNotFound),
            Err(e) => return Err(Error::Sql(e)),
        }
    }

    async fn get_all(&self) -> Result<Vec<response::Dispatch>, Error> {
        Ok(sqlx::query(
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
        .await?)
    }

    async fn get_by_nation(&self, nation: String) -> Result<Vec<response::Dispatch>, Error> {
        Ok(sqlx::query(
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
        .await?)
    }

    pub(crate) async fn get(
        &self,
        nation: Option<String>,
    ) -> Result<Vec<response::Dispatch>, Error> {
        match nation {
            Some(nation) => Ok(self.get_by_nation(nation).await?),
            None => Ok(self.get_all().await?),
        }
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
