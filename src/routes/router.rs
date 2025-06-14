use crate::controllers;
use crate::core::error;
use crate::core::state::AppState;
use crate::routes::{admin, dispatch, nations, queue, rmbpost, telegram, user};
use axum::error_handling::HandleErrorLayer;
use axum::routing::{options, patch};
use axum::{
    Router,
    extract::{MatchedPath, Request},
    http::{HeaderName, HeaderValue, Method, StatusCode},
    middleware,
    routing::{get, post},
};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{self, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::info_span;

pub(crate) async fn routes(
    state: AppState,
    dispatch_nations: Vec<String>,
    rmbpost_nations: Vec<String>,
) -> Router {
    let dispatch_nations = Box::leak(Box::new(dispatch_nations.join(",")));

    let rmbpost_nations = Box::leak(Box::new(rmbpost_nations.join(",")));

    // /dispatches/...
    let dispatch_router = Router::new()
        .route("/dispatches", get(dispatch::get_all).post(dispatch::post))
        .route(
            "/dispatches/{id}",
            get(dispatch::get)
                .put(dispatch::put)
                .delete(dispatch::delete),
        )
        .route_layer(
            ServiceBuilder::new().layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("dispatch-nations"),
                HeaderValue::from_static(dispatch_nations),
            )),
        );

    // /telegrams/...
    let telegram_router = Router::new().route(
        "/telegrams",
        get(telegram::get)
            .post(telegram::post)
            .delete(telegram::delete),
    );

    // /rmbposts/...
    let rmbpost_router = Router::new()
        .route("/rmbposts", post(rmbpost::post))
        .route_layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("rmbpost-nations"),
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
    let user_router = Router::new()
        .route("/users/{id}", get(user::get))
        .route("/users/username/{username}", get(user::get_by_username))
        .route("/users/me/password", patch(user::update_password))
        .route("/users/{id}/password", patch(admin::change_user_password));

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
        .with_state(state.clone())
        .route_layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    controllers::user::authenticate,
                ))
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
                        .expose_headers([
                            HeaderName::from_static("dispatch-nations"),
                            HeaderName::from_static("rmbpost-nations"),
                        ]),
                ),
        )
}
