use crate::core::error::Error;
use crate::core::state::AppState;
use crate::ns::rmbpost::NewRmbPost;
use crate::types::AuthorizedUser;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::{Extension, Json};

#[tracing::instrument(skip_all)]
pub(crate) async fn post(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthorizedUser>>,
    Json(params): Json<NewRmbPost>,
) -> Result<impl IntoResponse, Error> {
    match user {
        Some(user) => {
            if !user.claims.contains(&"rmbposts.create".to_string()) {
                return Err(Error::Unauthorized);
            }
        }
        None => return Err(Error::Unauthorized),
    }

    let status = state.rmbpost_controller.queue(params.clone()).await?;

    Ok((
        StatusCode::ACCEPTED,
        [(header::LOCATION, format!("/queue/rmbposts/{}", status.id))],
        Json(status),
    ))
}
