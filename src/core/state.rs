use crate::controllers::dispatch::DispatchController;
use crate::controllers::rmbpost::RmbpostController;
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
    pub(crate) user_controller: UserController,
    pub(crate) dispatch_controller: DispatchController,
    pub(crate) rmbpost_controller: RmbpostController,
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
            user_controller: UserController::new(pool.clone(), secret)?,
            dispatch_controller: DispatchController::new(pool.clone()),
            rmbpost_controller: RmbpostController::new(pool, rmbpost_sender),
        })
    }
}
