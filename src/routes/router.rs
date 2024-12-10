use crate::core::error;
use crate::core::state::AppState;
use crate::routes::{auth, dispatches, nations, queue, telegrams};
use crate::utils;
use axum::error_handling::HandleErrorLayer;
use axum::{
    extract::{MatchedPath, Request},
    http::{HeaderName, HeaderValue, Method, StatusCode},
    middleware,
    routing::{get, head, post, put},
    Router,
};
use std::str::FromStr;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{self, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
    validate_request::ValidateRequestHeaderLayer,
};
use tracing::info_span;

pub(crate) async fn routes(state: AppState) -> Router {
    let dispatch_nations = Box::leak(Box::new(state.client.get_nation_names().await.join(",")));

    let authorized_routes = Router::new()
        .route("/", post(dispatches::post))
        .route("/:id", put(dispatches::put).delete(dispatches::delete))
        .route_layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        )));

    // /dispatches/...
    let dispatch_router = Router::new()
        .route("/", get(dispatches::get_all))
        .route("/:id", get(dispatches::get))
        .nest("/", authorized_routes)
        .route_layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("allowed-nations"),
            HeaderValue::from_static(dispatch_nations),
        ));

    // /telegrams/...
    let telegram_router = Router::new()
        .route(
            "/",
            get(telegrams::get)
                .post(telegrams::post)
                .delete(telegrams::delete),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        ));

    // /queue/...
    let queue_router = Router::new().route("/dispatches/:id", get(queue::dispatch));

    // /nations/...
    let nation_router = Router::new().route("/:nation/dispatches", get(nations::dispatches::get));

    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/register", post(auth::register))
        .route("/login", post(auth::sign_in))
        .nest("/dispatches", dispatch_router)
        .nest("/telegrams", telegram_router)
        .nest("/queue", queue_router)
        .nest("/nations", nation_router)
        .with_state(state)
        .route_layer(
            ServiceBuilder::new()
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
                .layer(HandleErrorLayer::new(error::handle_middleware_errors))
                .buffer(128)
                .rate_limit(10, Duration::from_secs(1))
                .layer(
                    CorsLayer::new()
                        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_origin(cors::Any)
                        .expose_headers([HeaderName::from_str("allowed-nations").unwrap()]),
                ),
        )
}
