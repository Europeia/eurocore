use axum::extract::{Extension, Json, Path, State};
use axum::response::IntoResponse;
use tracing::instrument;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::types::request;
use crate::types::{AuthorizedUser, Username};

#[instrument(skip_all)]
pub(crate) async fn change_user_password(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Path(id): Path<i32>,
    Json(params): Json<request::UpdatePasswordData>,
) -> Result<impl IntoResponse, Error> {
    match user {
        Some(user) => {
            if !user.claims.contains(&"admin".to_string()) {
                return Err(Error::Unauthorized);
            }
        }
        None => return Err(Error::Unauthorized),
    }

    let username: Username = match state.user_controller.get_username_by_id(id).await {
        Ok(Some(user)) => user,
        Ok(None) => return Err(Error::InvalidUsername),
        Err(e) => return Err(e),
    };

    state
        .user_controller
        .update_password(&username, &params.new_password)
        .await?;

    Ok(Json("Password reset successfully"))
}
