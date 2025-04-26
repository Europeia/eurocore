use super::types::{Mode, Prepared, PrivateCommand, Unprepared};
use crate::core::error::Error;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NewRmbPost {
    pub(crate) nation: String,
    pub(crate) region: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug)]
pub(crate) struct IntermediateRmbPost {
    pub(crate) job_id: i32,
    pub(crate) nation: String,
    pub(crate) region: String,
    pub(crate) text: String,
}

impl IntermediateRmbPost {
    pub(crate) fn new(job_id: i32, nation: String, region: String, text: String) -> Self {
        Self {
            job_id,
            nation,
            region,
            text,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RmbPost<T: Serialize + PrivateCommand> {
    #[serde(rename = "c")]
    command: String,
    nation: String,
    region: String,
    text: String,
    mode: Mode,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    _state: PhantomData<T>,
}

impl<T: Serialize + PrivateCommand> RmbPost<T> {
    pub(crate) fn nation(&self) -> &str {
        &self.nation
    }
}

impl RmbPost<Unprepared> {
    fn new(nation: String, region: String, text: String) -> Self {
        Self {
            command: "rmbpost".to_string(),
            nation,
            region,
            text,
            mode: Mode::Prepare,
            token: None,
            _state: PhantomData,
        }
    }

    pub(crate) fn prepare(self, token: String) -> RmbPost<Prepared> {
        RmbPost {
            command: self.command,
            nation: self.nation,
            region: self.region,
            text: self.text,
            mode: Mode::Execute,
            token: Some(token),
            _state: PhantomData,
        }
    }
}

impl From<IntermediateRmbPost> for RmbPost<Unprepared> {
    fn from(intermediate: IntermediateRmbPost) -> Self {
        Self::new(intermediate.nation, intermediate.region, intermediate.text)
    }
}

#[derive(Debug)]
enum Action {
    Queue { post: NewRmbPost },
}

#[derive(Debug)]
pub(crate) struct Command {
    pub(crate) action: Action,
    pub(crate) tx: oneshot::Sender<Response>,
}

impl Command {
    pub(crate) fn new(rmbpost: IntermediateRmbPost, tx: oneshot::Sender<Response>) -> Self {
        Self { rmbpost, tx }
    }
}

#[derive(Debug)]
pub(crate) enum Response {
    Success,
    Error(Error),
}
