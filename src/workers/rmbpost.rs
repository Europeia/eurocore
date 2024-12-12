use crate::core::client::Client;
use crate::ns::rmbpost::{Command, IntermediateRmbPost};
use sqlx::PgPool;
use std::collections::VecDeque;
use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct RmbPostClient {
    pool: PgPool,
    client: Client,
    queue: VecDeque<IntermediateRmbPost>,

    rx: mpsc::Receiver<Command>,
}
