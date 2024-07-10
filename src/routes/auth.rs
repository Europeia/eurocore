use crate::core::state::AppState;
use crate::utils::auth::encode_jwt;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use bcrypt::verify;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct LoginData {
    pub(crate) nation: String,
    pub(crate) password: String,
}

pub(crate) async fn sign_in(
    State(state): State<AppState>,
    Json(user_data): Json<LoginData>,
) -> Result<Json<String>, StatusCode> {
    let user = match state.retrieve_user_by_nation(&user_data.nation).await {
        Ok(resp) => match resp {
            Some(user) => user,
            None => return Err(StatusCode::UNAUTHORIZED),
        },
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    if !verify(&user_data.password, &user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = encode_jwt(user.nation).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(token))
}
