use axum::extract::{Json, State};
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
    let dispatch_id = state.new_dispatch(params).await?;

    Ok(format!("Dispatch: {} added", dispatch_id))
}

#[instrument(skip(state))]
pub(crate) async fn edit_dispatch(
    State(mut state): State<AppState>,
    Json(params): Json<EditDispatchParams>,
) -> Result<String, Error> {
    let dispatch_id = state.edit_dispatch(params).await?;

    Ok(format!("Dispatch: {} edited", dispatch_id))
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