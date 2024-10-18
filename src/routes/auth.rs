use axum::extract::State;
use axum::Json;
use bcrypt::verify;
use serde::Deserialize;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::utils::auth::encode_jwt;

#[derive(Deserialize)]
pub(crate) struct LoginData {
    pub(crate) username: String,
    pub(crate) password: String,
}

pub(crate) async fn sign_in(
    State(state): State<AppState>,
    Json(user_data): Json<LoginData>,
) -> Result<Json<String>, Error> {
    let user = match state.retrieve_user_by_username(&user_data.username).await {
        Ok(resp) => match resp {
            Some(user) => user,
            None => return Err(Error::Unauthorized),
        },
        Err(e) => return Err(e),
    };

    match verify(&user_data.password, &user.password_hash).map_err(Error::Bcrypt)? {
        true => (),
        false => return Err(Error::Unauthorized),
    }

    let token = encode_jwt(user, &state.secret)?;

    Ok(Json(token))
}

pub(crate) async fn register(
    State(state): State<AppState>,
    Json(user_data): Json<LoginData>,
) -> Result<Json<String>, Error> {
    let password_hash = bcrypt::hash(&user_data.password, 12).map_err(Error::Bcrypt)?;

    let user = state
        .register_user(&user_data.username, &password_hash)
        .await?;

    let token = encode_jwt(user, &state.secret)?;

    Ok(Json(token))
}
