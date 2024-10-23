use axum::{
    extract::MatchedPath,
    http::Request,
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::trace::TraceLayer;
use tracing::info_span;

use crate::core::state::AppState;
use crate::{routes, utils};

pub(crate) async fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/register", post(routes::auth::register))
        .route("/login", post(routes::auth::sign_in))
        .route("/dispatch/:id", get(routes::dispatch::get_dispatch))
        .route("/dispatches", get(routes::dispatch::get_dispatches))
        .route(
            "/dispatches/:nation",
            get(routes::dispatch::get_dispatches_by_nation),
        )
        .route(
            "/dispatch",
            post(routes::dispatch::post_dispatch).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .route(
            "/dispatch/:id",
            put(routes::dispatch::edit_dispatch).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .route(
            "/dispatch/:id",
            delete(routes::dispatch::remove_dispatch).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .route(
            "/queue/dispatch/:id",
            get(routes::dispatch::get_queued_dispatch),
        )
        .route(
            "/telegram",
            get(routes::telegram::get_telegrams).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .route(
            "/telegram",
            post(routes::telegram::queue_telegram).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .route(
            "/telegram",
            delete(routes::telegram::delete_telegram).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .with_state(state)
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                info_span!(
                    "request",
                    method = ?request.method(),
                    matched_path,
                )
            }),
        )
}
