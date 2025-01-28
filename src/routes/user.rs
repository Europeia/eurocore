use axum::extract::{Extension, Path, State};
use axum::response::IntoResponse;
use axum::Json;
use bcrypt::verify;
use serde::Deserialize;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::types::request;
use crate::types::response;
use crate::utils::auth::{encode_jwt, AuthorizedUser};

#[derive(Deserialize)]
pub(crate) struct LoginData {
    pub(crate) username: String,
    pub(crate) password: String,
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(user_data): Json<LoginData>,
) -> Result<Json<response::Login>, Error> {
    let user = state
        .retrieve_user_by_username(&user_data.username)
        .await?
        .ok_or(Error::Unauthorized)?;

    match verify(&user_data.password, &user.password_hash).map_err(Error::Bcrypt)? {
        true => (),
        false => return Err(Error::Unauthorized),
    }

    let token = encode_jwt(&user, &state.secret)?;

    Ok(Json(response::Login::new(&user.username, &token)))
}

pub(crate) async fn register(
    State(state): State<AppState>,
    Json(user_data): Json<LoginData>,
) -> Result<Json<response::Login>, Error> {
    let password_hash = bcrypt::hash(&user_data.password, 12).map_err(Error::Bcrypt)?;

    let user = state
        .register_user(&user_data.username, &password_hash)
        .await?;

    let token = encode_jwt(&user, &state.secret)?;

    Ok(Json(response::Login::new(&user.username, &token)))
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, Error> {
    let username = state.get_user_by_id(id).await?.ok_or(Error::Unauthorized)?;

    Ok(Json(response::User::new(id, &username)))
}

pub(crate) async fn update_password(
    State(state): State<AppState>,
    Extension(user): Extension<AuthorizedUser>,
    Json(params): Json<request::UpdatePasswordData>,
) -> Result<impl IntoResponse, Error> {
    state
        .update_password(&user.username, &params.new_password)
        .await?;

    Ok(Json("Password reset successfully"))
}
