use crate::core::error::Error;
use crate::core::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use tracing::instrument;

#[instrument(skip(state))]
pub(crate) async fn dispatch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let status = state.dispatch_controller.get_status(id).await?;

    Ok(Json(status))
}

#[instrument(skip(state))]
pub(crate) async fn rmbpost(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let status = state.rmbpost_controller.get_status(id).await?;

    Ok(Json(status))
}
