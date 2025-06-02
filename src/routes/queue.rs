use crate::core::error::Error;
use crate::core::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;

#[tracing::instrument(skip_all)]
pub(crate) async fn dispatch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let status = state.dispatch_controller.get_status(id).await?;

    Ok(Json(status))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn rmbpost(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let status = state.rmbpost_controller.get_status(id).await?;

    Ok(Json(status))
}
