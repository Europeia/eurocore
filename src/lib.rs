pub(crate) mod controllers;
pub(crate) mod core;
pub(crate) mod ns;
pub(crate) mod routes;
pub(crate) mod sync;
pub(crate) mod token;
pub(crate) mod types;
pub(crate) mod utils;
pub(crate) mod workers;

use crate::controllers::{dispatch, rmbpost, telegram, user};
use crate::core::error::ConfigError as Error;
use crate::core::{config::Args, state::AppState};
use crate::routes::router;
use crate::sync::nations;
use crate::sync::ratelimiter;
use config::Config;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub async fn run() -> Result<(), Error> {
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("EUROCORE"))
        .build()?;

    let config = config.try_deserialize::<Args>()?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_new(config.log_level).unwrap_or_default())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        config.database_user,
        config.database_password,
        config.database_host,
        config.database_port,
        config.database_name
    );

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let ratelimiter = ratelimiter::new(
        50,
        Duration::from_secs(30),
        Duration::from_secs(30),
        Duration::from_secs(180),
        Duration::from_secs(60),
    );

    let dispatch_nations = nations::new(nations::Source::Str(config.dispatch_nations))?;
    let rmbpost_nations = nations::new(nations::Source::Str(config.rmbpost_nations))?;

    let dispatch_nation_names = dispatch_nations.list_nations().await.unwrap();
    let rmbpost_nation_names = rmbpost_nations.list_nations().await.unwrap();

    let dispatch_controller = dispatch::Controller::new(
        &config.user,
        "https://www.nationstates.net/cgi-bin/api.cgi",
        db_pool.clone(),
        ratelimiter.clone(),
        dispatch_nations,
    )?;

    let rmbpost_controller = rmbpost::Controller::new(
        &config.user,
        "https://www.nationstates.net/cgi-bin/api.cgi",
        db_pool.clone(),
        ratelimiter.clone(),
        rmbpost_nations,
    )?;

    let telegram_controller = telegram::Controller::new(
        &config.user,
        "https://www.nationstates.net/cgi-bin/api.cgi",
        config.telegram_client_key,
        ratelimiter.clone(),
        db_pool.clone(),
    )?;

    let user_controller = user::Controller::new(db_pool.clone(), config.secret)?;

    let state = AppState::new(
        user_controller,
        dispatch_controller,
        rmbpost_controller,
        telegram_controller,
    );

    sqlx::migrate!().run(&db_pool).await?;

    let app = router::routes(state.clone(), dispatch_nation_names, rmbpost_nation_names).await;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;

    tracing::debug!("listening on port {}", config.port);

    axum::serve(listener, app).await?;

    Ok(())
}
