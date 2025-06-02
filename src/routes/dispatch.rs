use axum::Extension;
use axum::extract::{Json, Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::ns::dispatch::{EditDispatch, NewDispatch};
use crate::types::AuthorizedUser;

#[tracing::instrument(skip_all)]
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let dispatch = state.dispatch_controller.get_one(id).await?;

    Ok(Json(dispatch))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn get_all(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let dispatches = state.dispatch_controller.get(None).await?;

    Ok(Json(dispatches))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn post(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Json(params): Json<NewDispatch>,
) -> Result<impl IntoResponse, Error> {
    let user = match user {
        Some(user) => {
            if !user.claims.contains(&"dispatches.create".to_string()) {
                return Err(Error::Unauthorized);
            }

            user
        }
        None => return Err(Error::Unauthorized),
    };

    let status = state.dispatch_controller.post(user, params).await?;

    Ok((
        StatusCode::ACCEPTED,
        [(header::LOCATION, format!("/queue/dispatches/{}", status.id))],
        Json(status),
    ))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn put(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Path(id): Path<i32>,
    Json(params): Json<EditDispatch>,
) -> Result<impl IntoResponse, Error> {
    let user = match user {
        Some(user) => {
            if !user.claims.contains(&"dispatches.edit".to_string()) {
                return Err(Error::Unauthorized);
            }

            user
        }
        None => return Err(Error::Unauthorized),
    };

    let status = state.dispatch_controller.put(user, id, params).await?;

    Ok((
        StatusCode::ACCEPTED,
        [(header::LOCATION, format!("/queue/dispatches/{}", status.id))],
        Json(status),
    ))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let user = match user {
        Some(user) => {
            if !user.claims.contains(&"dispatches.delete".to_string()) {
                return Err(Error::Unauthorized);
            }

            user
        }
        None => return Err(Error::Unauthorized),
    };

    let status = state.dispatch_controller.delete(user, id).await?;

    Ok((
        StatusCode::ACCEPTED,
        [(header::LOCATION, format!("/queue/dispatches/{}", status.id))],
        Json(status),
    ))
}
