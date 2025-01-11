use crate::core::error::Error;
use crate::core::state::AppState;
use crate::ns::rmbpost::NewRmbPost;
use crate::utils::auth::User;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};

pub(crate) async fn post(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(params): Json<NewRmbPost>,
) -> Result<impl IntoResponse, Error> {
    if !user.claims.contains(&"rmbposts.create".to_string()) {
        return Err(Error::Unauthorized);
    }

    let status = state.queue_rmbpost(params.clone()).await?;

    Ok(Json(status))
}
