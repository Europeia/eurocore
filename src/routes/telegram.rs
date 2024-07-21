use crate::core::error::Error;
use crate::core::state::AppState;
use axum::extract::{Json, State};
use std::collections::HashMap;
use tracing::instrument;

use crate::types::ns::TelegramParams;

#[instrument(skip(state))]
pub(crate) async fn get_telegrams(
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, Vec<String>>>, Error> {
    let telegrams = state.client.telegram_queue.list_telegrams().await;

    Ok(Json(telegrams))
}

#[instrument(skip(state))]
pub(crate) async fn queue_telegram(
    State(state): State<AppState>,
    Json(params): Json<Vec<TelegramParams>>,
) -> Result<String, Error> {
    for param in params {
        state.client.telegram_queue.queue_telegram(param).await;
    }

    Ok("Telegrams queued".to_string())
}

#[instrument(skip(state))]
pub(crate) async fn delete_telegram(
    State(state): State<AppState>,
    Json(params): Json<TelegramParams>,
) -> Result<String, Error> {
    state.client.telegram_queue.delete_telegram(params).await;

    Ok("Telegram deleted".to_string())
}
