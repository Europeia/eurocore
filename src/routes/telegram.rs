use axum::extract::{Json, State};
use axum::Extension;
use std::collections::HashMap;
use tokio::sync::oneshot;
use tracing::instrument;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::ns::telegram::{Command, Header, Operation, Params, Response};
use crate::types::response;
use crate::types::AuthorizedUser;

#[instrument(skip(state, user))]
pub(crate) async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
) -> Result<Json<HashMap<String, Vec<response::Telegram>>>, Error> {
    match user {
        Some(user) => {
            if !user.claims.contains(&"telegrams.read".to_string()) {
                return Err(Error::Unauthorized);
            }
        }
        None => return Err(Error::Unauthorized),
    }

    let (tx, rx) = oneshot::channel();

    state
        .telegram_sender
        .send(Command::new(Operation::List, tx))
        .await
        .unwrap();

    match rx.await {
        Ok(Response::List(telegrams)) => Ok(Json(telegrams)),
        Ok(_) => {
            tracing::error!("Invalid response from telegram worker");
            Err(Error::Internal)
        }
        Err(e) => {
            tracing::error!("Error listing telegrams: {}", e);
            Err(Error::Internal)
        }
    }
}

#[instrument(skip(state, user))]
pub(crate) async fn post(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Json(params): Json<Vec<Params>>,
) -> Result<String, Error> {
    match user {
        Some(user) => {
            if !user.claims.contains(&"telegrams.create".to_string()) {
                return Err(Error::Unauthorized);
            }
        }
        None => return Err(Error::Unauthorized),
    }

    for param in params {
        let (tx, rx) = oneshot::channel();

        state
            .telegram_sender
            .send(Command::new(Operation::Queue(param), tx))
            .await
            .unwrap();

        if let Err(e) = rx.await {
            tracing::error!("Error queueing telegram: {}", e);
        }
    }

    Ok("Telegrams queued".to_string())
}

#[instrument(skip(state, user))]
pub(crate) async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Json(params): Json<Header>,
) -> Result<String, Error> {
    match user {
        Some(user) => {
            if !user.claims.contains(&"telegrams.delete".to_string()) {
                return Err(Error::Unauthorized);
            }
        }
        None => return Err(Error::Unauthorized),
    }

    let (tx, rx) = oneshot::channel();

    state
        .telegram_sender
        .send(Command::new(Operation::Delete(params), tx))
        .await
        .unwrap();

    if let Err(e) = rx.await {
        tracing::error!("Error deleting telegram: {}", e);
    }

    Ok("Telegram deleted".to_string())
}
