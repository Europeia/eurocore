use axum::extract::{Extension, Json, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use tracing::instrument;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::utils::auth::AuthorizedUser;

#[derive(Deserialize)]
pub(crate) struct PasswordResetData {
    username: String,
    new_password: String,
}

#[instrument(skip_all)]
pub(crate) async fn change_user_password(
    State(state): State<AppState>,
    Extension(user): Extension<AuthorizedUser>,
    Json(params): Json<PasswordResetData>,
) -> Result<impl IntoResponse, Error> {
    if !user.claims.contains(&"admin".to_string()) {
        return Err(Error::Unauthorized);
    }

    if let None = state.retrieve_user_by_username(&params.username).await? {
        return Err(Error::InvalidUsername);
    }

    state
        .update_password(&params.username, &params.new_password)
        .await?;

    Ok(Json("Password reset successfully"))
}
