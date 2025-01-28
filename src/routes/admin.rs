use axum::extract::{Extension, Json, Path, State};
use axum::response::IntoResponse;
use tracing::instrument;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::types::request;
use crate::utils::auth::AuthorizedUser;

#[instrument(skip_all)]
pub(crate) async fn change_user_password(
    State(state): State<AppState>,
    Extension(user): Extension<AuthorizedUser>,
    Path(id): Path<i32>,
    Json(params): Json<request::UpdatePasswordData>,
) -> Result<impl IntoResponse, Error> {
    if !user.claims.contains(&"admin".to_string()) {
        return Err(Error::Unauthorized);
    }

    let username = match state.get_user_by_id(id).await {
        Ok(Some(user)) => user,
        Ok(None) => return Err(Error::InvalidUsername),
        Err(e) => return Err(e),
    };

    state
        .update_password(&username, &params.new_password)
        .await?;

    Ok(Json("Password reset successfully"))
}
