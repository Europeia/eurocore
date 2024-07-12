use crate::core::error::Error;
use crate::core::state::AppState;
use axum::extract::{Json, State};
use tracing::instrument;

use crate::types::ns::{Telegram, TelegramParams};

#[instrument(skip(state))]
pub(crate) async fn list_telegrams(State(state): State<AppState>) -> Result<String, Error> {
    let telegrams = state.client.list_telegrams().await?;

    Ok(telegrams)
}

#[instrument(skip(state))]
pub(crate) async fn queue_telegram(
    State(mut state): State<AppState>,
    Json(params): Json<TelegramParams>,
) -> Result<String, Error> {
    let telegram = Telegram::from_params(&state.client.telegram_client_key, params);

    state.client.queue_telegram(telegram).await;

    Ok("Telegram queued".to_string())
}

#[instrument(skip(state))]
pub(crate) async fn delete_telegram(
    State(mut state): State<AppState>,
    Json(params): Json<TelegramParams>,
) -> Result<String, Error> {
    let telegram = Telegram::from_params(&state.client.telegram_client_key, params);

    state.client.delete_telegram(telegram).await;

    Ok("Telegram deleted".to_string())
}
