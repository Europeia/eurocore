use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::types::AuthorizedUser;
use crate::types::request;
use crate::types::response;

#[tracing::instrument(skip_all)]
pub(crate) async fn register(
    State(state): State<AppState>,
    Json(input): Json<request::LoginData>,
) -> Result<impl IntoResponse, Error> {
    let (user, token) = state
        .user_controller
        .register(&input.username, &input.password)
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        [(header::LOCATION, format!("/users/{}", user.id))],
        Json(response::Login::new(&user.username, &token)),
    ))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn login(
    State(state): State<AppState>,
    Json(input): Json<request::LoginData>,
) -> Result<Json<response::Login>, Error> {
    let (user, token) = state
        .user_controller
        .login(&input.username, &input.password)
        .await?;

    Ok(Json(response::Login::new(&user.username, &token)))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let username = state
        .user_controller
        .get_username_by_id(id)
        .await?
        .ok_or(Error::InvalidUsername)?;

    Ok(Json(response::User::new(id, &username)))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn get_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let user = match state
        .user_controller
        .get_user_by_username(&username)
        .await?
    {
        Some(user) => user,
        None => return Err(Error::Unauthorized),
    };

    Ok(Json(response::User::new(user.id, &user.username)))
}

#[tracing::instrument(skip_all)]
pub(crate) async fn update_password(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Json(params): Json<request::UpdatePasswordData>,
) -> Result<impl IntoResponse, Error> {
    let user = match user {
        Some(user) => user,
        None => return Err(Error::Unauthorized),
    };

    state
        .user_controller
        .update_password(&user.username, &params.new_password)
        .await?;

    Ok(Json("Password reset successfully"))
}
