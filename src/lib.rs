pub(crate) mod core;
pub(crate) mod ns;
pub(crate) mod routes;
pub(crate) mod types;
pub(crate) mod utils;
pub(crate) mod workers;

use axum::{
    extract::MatchedPath,
    http::Request,
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use config::Config;
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use workers::dispatch::DispatchClient;

use crate::core::error::ConfigError as Error;
use crate::core::{client::Client, config::Args, state::AppState};
use crate::ns::nation::{create_nations_map, NationList};
use crate::utils::ratelimiter::Ratelimiter;
use crate::workers::telegram::TelegramClient;

pub async fn run() -> Result<(), Error> {
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("EUROCORE"))
        .build()?;

    let config = config.try_deserialize::<Args>()?;

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

    let ratelimiter = Ratelimiter::new(
        50,
        Duration::from_millis(30_050),
        Duration::from_millis(30_050),
        Duration::from_millis(180_050),
        Duration::from_secs(60),
    );

    let client = Client::new(
        &config.user,
        NationList::new(create_nations_map(&config.nations)),
        ratelimiter,
    )?;

    let (telegram_sender, telegram_receiver) = tokio::sync::mpsc::channel(8);

    let mut telegram_client = TelegramClient::new(
        client.clone(),
        config.telegram_client_key,
        telegram_receiver,
    )?;

    let (dispatch_sender, dispatch_receiver) = tokio::sync::mpsc::channel(8);

    let mut dispatch_client = DispatchClient::new(client.clone(), dispatch_receiver)?;

    let state = AppState::new(
        &database_url,
        client,
        config.secret,
        telegram_sender,
        dispatch_sender,
    )
    .await?;

    tokio::spawn(async move { telegram_client.run().await });

    tokio::spawn(async move { dispatch_client.run().await });

    sqlx::migrate!().run(&state.pool.clone()).await?;

    let app = Router::new()
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

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;

    tracing::debug!("listening on port {}", config.port);

    axum::serve(listener, app).await?;

    Ok(())
}
