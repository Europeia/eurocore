use axum::extract::{Json, State};
use axum::Extension;
use std::collections::HashMap;
use tracing::instrument;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::ns::telegram::{TelegramHeader, TelegramParams};
use crate::types::response;
use crate::utils::auth::User;

#[instrument(skip(state, user))]
pub(crate) async fn get_telegrams(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<HashMap<String, Vec<response::Telegram>>>, Error> {
    if !user.claims.contains(&"telegrams.read".to_string()) {
        return Err(Error::Unauthorized);
    }

    let telegrams = state.client.telegram_queue.list_telegrams().await;

    Ok(Json(telegrams))
}

#[instrument(skip(state, user))]
pub(crate) async fn queue_telegram(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(params): Json<Vec<TelegramParams>>,
) -> Result<String, Error> {
    if !user.claims.contains(&"telegrams.create".to_string()) {
        return Err(Error::Unauthorized);
    }

    for param in params {
        state.client.telegram_queue.queue_telegram(param).await;
    }

    Ok("Telegrams queued".to_string())
}

#[instrument(skip(state, user))]
pub(crate) async fn delete_telegram(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(params): Json<TelegramHeader>,
) -> Result<String, Error> {
    if !user.claims.contains(&"telegrams.delete".to_string()) {
        return Err(Error::Unauthorized);
    }

    state.client.telegram_queue.delete_telegram(params).await;

    Ok("Telegram deleted".to_string())
}
