pub(crate) mod core;
pub(crate) mod routes;
pub(crate) mod types;
pub(crate) mod utils;

use axum::{
    extract::MatchedPath,
    http::Request,
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use config::Config;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::core::error::ConfigError as Error;
use crate::core::telegram::telegram_loop;
use crate::core::{config::Args, state::AppState};

pub async fn run() -> Result<(), Error> {
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("EUROCORE"))
        .build()
        .map_err(Error::Config)?;

    let config = config.try_deserialize::<Args>().map_err(Error::Config)?;

    let database_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        config.database_user,
        config.database_password,
        config.database_host,
        config.database_port,
        config.database_name
    );

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_new(config.log_level).unwrap_or_default())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::new(
        &database_url,
        &config.user,
        config.ns_nation,
        config.ns_password,
        config.secret,
        config.telegram_client_key,
    )
    .await?;

    sqlx::migrate!()
        .run(&state.pool.clone())
        .await
        .map_err(Error::DatabaseMigration)?;

    let telegram_state = state.clone();

    tokio::spawn(async move {
        telegram_loop(telegram_state).await;
    });

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/register", post(routes::auth::register))
        .route("/login", post(routes::auth::sign_in))
        .route(
            "/dispatch",
            post(routes::dispatch::post_dispatch).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .route(
            "/dispatch",
            put(routes::dispatch::edit_dispatch).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
        )
        .route(
            "/dispatch",
            delete(routes::dispatch::remove_dispatch).layer(middleware::from_fn_with_state(
                state.clone(),
                utils::auth::authorize,
            )),
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
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .map_err(Error::IO)?;

    tracing::debug!("listening on port {}", config.port);

    axum::serve(listener, app).await.map_err(Error::IO)?;

    Ok(())
}
