pub(crate) mod core;
pub(crate) mod utils;
pub(crate) mod types;
pub(crate) mod routes;

use axum::{
    Router,
    extract::MatchedPath,
    http::Request,
    routing::{get, post, put, delete},
};
use config::Config;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tower_http::trace::TraceLayer;
use tracing::info_span;

use crate::core::{
    config::Args,
    state::AppState
};
use crate::core::error::Error;
use crate::routes::dispatch::{edit_dispatch, post_dispatch, remove_dispatch};

pub async fn run() -> Result<(), Error> {
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("EUROCORE"))
        .build()
        .map_err(Error::Config)?;

    let config = config
        .try_deserialize::<Args>()
        .map_err(Error::Config)?;

    let database_url = format!("postgresql://{}:{}@{}:{}/{}",
        config.database_user,
        config.database_password,
        config.database_host,
        config.database_port,
        config.database_name
    );

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_new(config.log_level)
                .unwrap_or_default()
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::new(
        &database_url,
        &config.user,
        config.ns_nation,
        config.ns_password,
    ).await?;

    sqlx::migrate!()
        .run(&state.pool.clone())
        .await
        .map_err(Error::DatabaseMigration)?;

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/dispatch", post(post_dispatch))
        .route("/dispatch", put(edit_dispatch))
        .route("/dispatch", delete(remove_dispatch))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "request",
                        method = ?request.method(),
                        matched_path,
                    )
                })
        );

    let listener = tokio::net::TcpListener::
        bind(format!("0.0.0.0:{}", config.port))
        .await
        .map_err(Error::IO)?;

    tracing::debug!("listening on port {}", config.port);

    axum::serve(listener, app)
        .await
        .map_err(Error::IO)?;

    Ok(())
}