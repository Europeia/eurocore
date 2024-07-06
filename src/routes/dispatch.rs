use axum::{
    extract::{Json, State},
};
use tracing::instrument;
use crate::core::error::Error;
use crate::core::state::AppState;

use crate::types::ns::{
    NewDispatchParams,
    EditDispatchParams,
    RemoveDispatchParams,
    Dispatch,
};

#[instrument(skip(state))]
pub(crate) async fn post_dispatch(
    State(mut state): State<AppState>,
    Json(params): Json<NewDispatchParams>,
) -> Result<String, Error> {
    tracing::debug!("Creating dispatch from params");
    let dispatch = Dispatch::try_from_new_params(params, &state.client.nation)?;

    tracing::debug!("Adding dispatch");
    let message = state.client.add_dispatch(dispatch).await?;

    Ok(format!("Dispatch: {} added", message))
}

#[instrument(skip(state))]
pub(crate) async fn edit_dispatch(
    State(mut state): State<AppState>,
    Json(params): Json<EditDispatchParams>,
) -> Result<String, Error> {
    tracing::debug!("Creating dispatch from params");
    let dispatch = Dispatch::try_from_edit_params(params, &state.client.nation)?;

    tracing::debug!("Editing dispatch");
    let message = state.client.add_dispatch(dispatch).await?;

    Ok(format!("Dispatch: {} edited", message))
}

#[instrument(skip(state))]
pub(crate) async fn remove_dispatch(
    State(mut state): State<AppState>,
    Json(params): Json<RemoveDispatchParams>,
) -> Result<String, Error> {
    tracing::debug!("Creating dispatch from params");
    let dispatch = Dispatch::from_remove_params(params, &state.client.nation);

    tracing::debug!("Removing dispatch");
    state.client.delete_dispatch(dispatch).await?;

    Ok("Dispatch removed".to_string())
}