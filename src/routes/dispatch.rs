use crate::core::error::Error;
use crate::core::state::AppState;
use axum::extract::{Json, Path, State};
use axum::Extension;
use tracing::instrument;

use crate::ns::dispatch::{EditDispatch, NewDispatch};
use crate::types::response::{Dispatch, DispatchHeader};
use crate::utils::auth::User;

#[instrument(skip(state))]
pub(crate) async fn get_dispatch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Dispatch>, Error> {
    let dispatch = state.get_dispatch(id).await?;

    Ok(Json(dispatch))
}

#[instrument(skip(state))]
pub(crate) async fn get_dispatches(
    State(state): State<AppState>,
) -> Result<Json<Vec<DispatchHeader>>, Error> {
    let dispatches = state.get_dispatches(None).await?;

    Ok(Json(dispatches))
}

#[instrument(skip(state))]
pub(crate) async fn get_dispatches_by_nation(
    State(state): State<AppState>,
    Path(nation): Path<String>,
) -> Result<Json<Vec<DispatchHeader>>, Error> {
    let dispatches = state.get_dispatches(Some(nation)).await?;

    Ok(Json(dispatches))
}

#[instrument(skip(state, user))]
pub(crate) async fn post_dispatch(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(params): Json<NewDispatch>,
) -> Result<Json<DispatchHeader>, Error> {
    if !user.claims.contains(&"dispatches.create".to_string()) {
        return Err(Error::Unauthorized);
    }

    let dispatch = state.new_dispatch(params, &user.username).await?;

    Ok(Json(dispatch))
}

#[instrument(skip(state, user))]
pub(crate) async fn edit_dispatch(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
    Json(params): Json<EditDispatch>,
) -> Result<Json<DispatchHeader>, Error> {
    if !user.claims.contains(&"dispatches.edit".to_string()) {
        return Err(Error::Unauthorized);
    }

    let dispatch = state.edit_dispatch(id, params, &user.username).await?;

    Ok(Json(dispatch))
}

#[instrument(skip(state, user))]
pub(crate) async fn remove_dispatch(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> Result<Json<DispatchHeader>, Error> {
    if !user.claims.contains(&"dispatches.delete".to_string()) {
        return Err(Error::Unauthorized);
    }

    let dispatch = state.remove_dispatch(id).await?;

    Ok(Json(dispatch))
}
