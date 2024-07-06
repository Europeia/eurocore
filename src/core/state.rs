use sqlx::postgres::{PgPool, PgPoolOptions};
use crate::core::error::Error;
use crate::core::client::Client;

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
        password: String
    ) -> Result<Self, Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(Error::DatabaseConnectionFailure)?;

        let client = Client::new(user, nation, password)?;

        Ok(AppState {
            pool,
            client,
        })
    }
}