pub(super) mod dispatches {
    use crate::core::error::Error;
    use crate::core::state::AppState;
    use axum::Json;
    use axum::extract::{Path, State};
    use axum::response::IntoResponse;

    #[tracing::instrument(skip_all)]
    pub(crate) async fn get(
        State(state): State<AppState>,
        Path(nation): Path<String>,
    ) -> Result<impl IntoResponse, Error> {
        let dispatches = state.dispatch_controller.get(Some(nation)).await?;

        Ok(Json(dispatches))
    }
}
