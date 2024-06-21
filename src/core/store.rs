use sqlx::postgres::{PgPool, PgPoolOptions};
use crate::core::error::Error;
use crate::core::ns::Client;

pub(crate) struct Store {
    pub(crate) pool: PgPool,
    pub(crate) client: Client,
}

impl Store {
    pub(crate) async fn new(database_url: &str, user: &str) -> Result<Self, Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| Error::DatabaseConnectionFailure(e))?;

        let client = Client::new(user)?;

        Ok(Store {
            pool,
            client
        })
    }
}