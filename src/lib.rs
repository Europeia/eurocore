pub(crate) mod core;
pub(crate) mod ns;
pub(crate) mod routes;
pub(crate) mod types;
pub(crate) mod utils;
pub(crate) mod workers;

use config::Config;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::core::error::ConfigError as Error;
use crate::core::{client::Client, config::Args, state::AppState};
use crate::ns::nation::{create_nations_map, NationList};
use crate::utils::ratelimiter::Ratelimiter;
use crate::workers::{dispatch::DispatchClient, telegram::TelegramClient};

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

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

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

    let mut dispatch_client =
        DispatchClient::new(db_pool.clone(), client.clone(), dispatch_receiver);

    let state = AppState::new(
        db_pool.clone(),
        config.secret,
        telegram_sender,
        dispatch_sender,
    )
    .await;

    tokio::spawn(async move { telegram_client.run().await });

    tokio::spawn(async move { dispatch_client.run().await });

    sqlx::migrate!().run(&db_pool.clone()).await?;

    let app = routes::router::routes(state.clone()).await;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;

    tracing::debug!("listening on port {}", config.port);

    axum::serve(listener, app).await?;

    Ok(())
}
