use axum::Extension;
use axum::extract::{Json, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use sqlx;
use tokio::sync::oneshot;
use tracing::instrument;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::ns::dispatch::{Command, EditDispatch, IntermediateDispatch, NewDispatch, Response};
use crate::types::AuthorizedUser;
use crate::types::response::DispatchStatus;

#[instrument(skip_all)]
pub(crate) async fn head(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let mut headers = HeaderMap::new();

    headers.insert(
        "X-Nations",
        HeaderValue::from_str(&state.client.get_dispatch_nation_names().await.join(","))?,
    );

    Ok((headers, StatusCode::NO_CONTENT))
}

#[instrument(skip(state))]
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let dispatch = state.dispatch_controller.get_one(id).await?;

    Ok(Json(dispatch))
}

#[instrument(skip(state))]
pub(crate) async fn get_all(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let dispatches = state.dispatch_controller.get(None).await?;

    Ok(Json(dispatches))
}

#[instrument(skip(state, user))]
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

    let job = state
        .dispatch_controller
        .queue("add", sqlx::types::Json(params.clone()))
        .await?;

    let dispatch = IntermediateDispatch::add(job.id, user.username, params)?;

    let (tx, rx) = oneshot::channel();

    state
        .dispatch_sender
        .send(Command::new(dispatch, tx))
        .await
        .unwrap();

    return_queued_dispatch(rx, job).await
}

#[instrument(skip(state, user))]
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

    let nation = state.dispatch_controller.get_nation(id).await?;

    let job = state
        .dispatch_controller
        .queue("edit", sqlx::types::Json(params.clone()))
        .await?;

    let dispatch = IntermediateDispatch::edit(job.id, user.username, id, nation, params)?;

    let (tx, rx) = oneshot::channel();

    state
        .dispatch_sender
        .send(Command::new(dispatch, tx))
        .await
        .unwrap();

    return_queued_dispatch(rx, job).await
}

#[instrument(skip(state, user))]
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

    let nation = state.dispatch_controller.get_nation(id).await?;

    let job = state
        .dispatch_controller
        .queue("remove", sqlx::types::Json(id))
        .await?;

    let dispatch = IntermediateDispatch::delete(job.id, user.username, id, nation);

    let (tx, rx) = oneshot::channel();

    state
        .dispatch_sender
        .send(Command::new(dispatch, tx))
        .await
        .unwrap();

    return_queued_dispatch(rx, job).await
}

async fn return_queued_dispatch(
    rx: oneshot::Receiver<Response>,
    job: DispatchStatus,
) -> Result<impl IntoResponse, Error> {
    match rx.await {
        Ok(_) => Ok((
            StatusCode::ACCEPTED,
            [(header::LOCATION, format!("/queue/dispatches/{}", job.id))],
            Json(job),
        )),
        Err(_e) => Err(Error::Internal),
    }
}
