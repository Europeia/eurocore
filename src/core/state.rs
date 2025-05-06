use crate::controllers::{dispatch, rmbpost, telegram, user};
use crate::core::client::Client;
use crate::core::error::ConfigError;
use sqlx::postgres::PgPool;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) client: Client,
    pub(crate) user_controller: user::Controller,
    pub(crate) dispatch_controller: dispatch::Controller,
    pub(crate) rmbpost_controller: rmbpost::Controller,
    pub(crate) telegram_controller: telegram::Controller,
}

impl AppState {
    pub(crate) async fn new(
        pool: PgPool,
        client: Client,
        user_controller: user::Controller,
        dispatch_controller: dispatch::Controller,
        rmbpost_controller: rmbpost::Controller,
        telegram_controller: telegram::Controller,
    ) -> Result<Self, ConfigError> {
        Ok(AppState {
            pool: pool.clone(),
            client,
            user_controller,
            dispatch_controller,
            rmbpost_controller,
            telegram_controller,
        })
    }
}
