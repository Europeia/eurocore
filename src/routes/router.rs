use axum::extract::State;
use axum::middleware::Next;
use axum::routing::options;
use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    http::{HeaderName, Method},
    middleware,
    response::Response,
    routing::{delete, get, head, post, put},
    Router,
};
use std::str::FromStr;
use tower_http::{cors, trace::TraceLayer};
use tracing::info_span;

use crate::core::error::Error;
use crate::core::state::AppState;
use crate::{routes, utils};

async fn add_dispatch_nations_header(
    State(state): State<AppState>,
    mut response: Response,
) -> Result<Response, Error> {
    let nations = state.client.get_nation_names().await.join(",");

    response
        .headers_mut()
        .insert(HeaderName::from_str("allowed-nations")?, nations.parse()?);

    Ok(response)
}

pub(crate) async fn routes(state: AppState) -> Router {
    // /dispatches/...
    let dispatches_router = Router::new()
        .route("/", get(routes::dispatch::get_dispatches))
        .route("/:nation", get(routes::dispatch::get_dispatches_by_nation))
        .layer(
            cors::CorsLayer::new()
                .allow_methods([Method::GET])
                .allow_origin(cors::Any),
        );

    // /dispatch/...
    let dispatch_router = Router::new()
        .route("/", head(routes::dispatch::head_dispatch))
        .route("/:id", get(routes::dispatch::get_dispatch))
        .route("/", post(routes::dispatch::post_dispatch))
        .route("/:id", put(routes::dispatch::edit_dispatch))
        .route("/:id", delete(routes::dispatch::remove_dispatch))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        ))
        .layer(
            cors::CorsLayer::new()
                .allow_methods([Method::POST, Method::PUT, Method::DELETE])
                .allow_origin(cors::Any),
        )
        .layer(middleware::map_response_with_state(
            state.clone(),
            add_dispatch_nations_header,
        ));

    // /telegram/...
    let telegram_router = Router::new()
        .route(
            "/",
            get(routes::telegram::get_telegrams)
                .post(routes::telegram::queue_telegram)
                .delete(routes::telegram::delete_telegram),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        ))
        .layer(
            cors::CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_origin(cors::Any),
        );

    // /queue/...
    let queue_router =
        Router::new().route("/dispatch/:id", get(routes::dispatch::get_queued_dispatch));

    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/register", post(routes::auth::register))
        .route("/login", post(routes::auth::sign_in))
        .nest("/dispatches", dispatches_router)
        .nest("/dispatch", dispatch_router)
        .nest("/telegram", telegram_router)
        .nest("/queue", queue_router)
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
