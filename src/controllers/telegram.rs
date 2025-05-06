use crate::core::error::{ConfigError, Error};
use crate::ns::telegram::{Command, Header, Operation, Params, Response};
use crate::sync::ratelimiter;
use crate::types::response;
use crate::workers;
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug)]
pub(crate) struct Controller {
    pool: PgPool,
    tx: mpsc::Sender<Command>,
}

impl Controller {
    pub(crate) fn new(
        user_agent: &str,
        url: &str,
        key: String,
        limiter: ratelimiter::Sender,
        pool: PgPool,
    ) -> Result<Self, ConfigError> {
        let (tx, mut client) = workers::telegram::new(user_agent, url, key, limiter)?;

        tokio::spawn(async move {
            client.run().await;
        });

        Ok(Self { pool, tx })
    }

    pub(crate) async fn get(&mut self) -> Result<HashMap<String, Vec<response::Telegram>>, Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.tx.send(Command::list(tx)).await {
            tracing::error!("{}", e);
            return Err(Error::Internal);
        }

        match rx.await {
            Ok(Response::List(list)) => Ok(list),
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::error!("{}", e);
                Err(Error::Internal)
            }
        }
    }

    pub(crate) async fn queue(&mut self, params: Vec<Params>) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.tx.send(Command::queue(params, tx)).await {
            tracing::error!("{}", e);
            return Err(Error::Internal);
        }

        match rx.await {
            Ok(Response::Ok) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::error!("{}", e);
                Err(Error::Internal)
            }
        }
    }

    pub(crate) async fn delete(&mut self, header: Header) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.tx.send(Command::delete(header, tx)).await {
            tracing::error!("{}", e);
            return Err(Error::Internal);
        }

        match rx.await {
            Ok(Response::Ok) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => {
                tracing::error!("{}", e);
                Err(Error::Internal)
            }
        }
    }
}
