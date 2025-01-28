use crate::core::error;
use crate::core::state::AppState;
use crate::routes::{admin, dispatch, nations, queue, rmbpost, telegram, user};
use crate::utils;
use axum::error_handling::HandleErrorLayer;
use axum::routing::patch;
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
        .route("/dispatches", post(dispatch::post))
        .route(
            "/dispatches/{id}",
            put(dispatch::put).delete(dispatch::delete),
        )
        .route_layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        )));

    // /dispatches/...
    let dispatch_router = Router::new()
        .route("/dispatches", get(dispatch::get_all))
        .route("/dispatches/{id}", get(dispatch::get))
        .merge(authorized_routes)
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

    // /users/...
    let user_router = Router::new().route("/users/{id}", get(user::get)).route(
        "/users/{id}",
        patch(user::update_password).route_layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        )),
    );

    // /admin/...
    let admin_router = Router::new()
        .route("/admin/reset_password", patch(admin::change_user_password))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            utils::auth::authorize,
        ));

    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/heartbeat", get(|| async { StatusCode::OK }))
        .route("/register", post(user::register))
        .route("/login", post(user::login))
        .merge(dispatch_router)
        .merge(telegram_router)
        .merge(rmbpost_router)
        .merge(queue_router)
        .merge(nation_router)
        .merge(user_router)
        .merge(admin_router)
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
                        .allow_methods([
                            Method::HEAD,
                            Method::GET,
                            Method::POST,
                            Method::PUT,
                            Method::DELETE,
                        ])
                        .allow_origin(cors::Any)
                        .expose_headers([HeaderName::from_str("allowed-nations").unwrap()]),
                ),
        )
}
