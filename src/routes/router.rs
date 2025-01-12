use crate::core::error;
use crate::core::state::AppState;
use crate::routes::{auth, dispatch, nations, queue, rmbpost, telegram};
use crate::utils;
use axum::error_handling::HandleErrorLayer;
use axum::{
    extract::{MatchedPath, Request},
    http::{HeaderName, HeaderValue, Method, StatusCode},
    middleware,
    routing::{get, post, put},
    Router,
};
use std::str::FromStr;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{self, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::info_span;

pub(crate) async fn routes(state: AppState) -> Router {
    let dispatch_nations = Box::leak(Box::new(
        state.client.get_dispatch_nation_names().await.join(","),
    ));

    let rmbpost_nations = Box::leak(Box::new(
        state.client.get_rmbpost_nation_names().await.join(","),
    ));

    let authorized_routes = Router::new()
        .route("/", post(dispatch::post))
        .route("/{id}", put(dispatch::put).delete(dispatch::delete))
        .route_layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        )));

    // /dispatches/...
    let dispatch_router = Router::new()
        .route("/dispatches", get(dispatch::get_all))
        .route("/dispatches/{id}", get(dispatch::get))
        .nest("/dispatches/", authorized_routes)
        .route_layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("allowed-nations"),
            HeaderValue::from_static(dispatch_nations),
        ));

    // /telegrams/...
    let telegram_router = Router::new()
        .route(
            "/telegrams",
            get(telegram::get)
                .post(telegram::post)
                .delete(telegram::delete),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        ));

    // /rmbposts/...
    let rmbpost_router = Router::new()
        .route("/rmbposts", post(rmbpost::post))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        ))
        .route_layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("allowed-nations"),
            HeaderValue::from_static(rmbpost_nations),
        ));

    // /queue/...
    let queue_router = Router::new()
        .route("/queue/dispatches/{id}", get(queue::dispatch))
        .route("/queue/rmbposts/{id}", get(queue::rmbpost));

    // /nations/...
    let nation_router = Router::new().route(
        "/nations/{nation}/dispatches",
        get(nations::dispatches::get),
    );

    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/heartbeat", get(|| async { StatusCode::OK }))
        .route("/register", post(auth::register))
        .route("/login", post(auth::sign_in))
        .merge(dispatch_router)
        .merge(telegram_router)
        .merge(rmbpost_router)
        .merge(queue_router)
        .merge(nation_router)
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
