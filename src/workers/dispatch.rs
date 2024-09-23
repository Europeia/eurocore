use crate::core::client::Client;
use crate::core::error::ConfigError;
use crate::ns::dispatch::{Action, Command, IntermediateDispatch, Response};
use crate::utils::ratelimiter::Target;
use crate::workers::PERIOD;
use regex::Regex;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct DispatchClient {
    client: Client,
    id_regex: Regex,
    queue: VecDeque<IntermediateDispatch>,

    rx: mpsc::Receiver<Command>,
}

impl DispatchClient {
    pub(crate) fn new(client: Client, rx: mpsc::Receiver<Command>) -> Result<Self, ConfigError> {
        Ok(Self {
            client,
            id_regex: Regex::new(r#"(\d+)"#)?,
            rx,
            queue: VecDeque::new(),
        })
    }

    async fn post(&mut self) {
        if let Some(dispatch) = self.get_action().await {
            match self.client.post_dispatch(dispatch).await {
                Ok(val) => tracing::debug!("{}", val),
                Err(e) => tracing::error!("{}", e),
            }
        }
    }

    async fn get_action(&mut self) -> Option<IntermediateDispatch> {
        for (index, dispatch) in self.queue.iter().enumerate() {
            if self.post_in(dispatch).await <= PERIOD {
                return Some(self.queue.remove(index).unwrap());
            }
        }

        None
    }

    async fn post_in(&self, dispatch: &IntermediateDispatch) -> Duration {
        match dispatch.action {
            Action::Add { .. } => {
                self.client
                    .ratelimiter
                    .peek_ratelimit(Target::Restricted(&dispatch.nation))
                    .await
            }
            Action::Edit { .. } | Action::Remove { .. } => {
                self.client
                    .ratelimiter
                    .peek_ratelimit(Target::Standard)
                    .await
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(e) => match e {
                    mpsc::error::TryRecvError::Empty => {
                        tracing::debug!("Dispatch channel empty")
                    }
                    mpsc::error::TryRecvError::Disconnected => {
                        tracing::error!("Dispatch channel disconnected");
                        break;
                    }
                },
                Ok(command) => {
                    self.queue.push_back(command.dispatch);

                    if let Err(e) = command.tx.send(Response::Success) {
                        tracing::error!("Error sending dispatch response, {:?}", e);
                    }
                }
            }

            self.post().await;

            tokio::time::sleep(PERIOD).await;
        }
    }
}
