use axum::Extension;
use axum::extract::{Json, State};
use std::collections::HashMap;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::ns::telegram::{Header, Params};
use crate::types::AuthorizedUser;
use crate::types::response;

#[tracing::instrument(skip_all)]
pub(crate) async fn get(
    State(mut state): State<AppState>,
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

    let telegrams = state.telegram_controller.get().await?;

    Ok(Json(telegrams))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn post(
    State(mut state): State<AppState>,
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

    state.telegram_controller.queue(params).await?;

    Ok("Telegrams queued".to_string())
}

#[tracing::instrument(skip_all)]
pub(crate) async fn delete(
    State(mut state): State<AppState>,
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

    state.telegram_controller.delete(params).await?;

    Ok("Telegram deleted".to_string())
}
