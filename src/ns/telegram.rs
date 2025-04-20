use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::oneshot;

use crate::types::response;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Telegram {
    #[serde(skip)]
    pub(crate) sender: String,
    #[serde(rename = "a")]
    action: String,
    #[serde(rename = "client")]
    client_key: String,
    #[serde(rename = "tgid")]
    pub(crate) telegram_id: String,
    #[serde(rename = "key")]
    secret_key: String,
    #[serde(rename = "to")]
    pub(crate) recipient: String,
    #[serde(skip)]
    pub(crate) tg_type: TgType,
}

impl std::fmt::Display for Telegram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.telegram_id, self.recipient)
    }
}

impl Telegram {
    pub(crate) fn header(&self) -> Header {
        Header {
            recipient: self.recipient.clone(),
            telegram_id: self.telegram_id.clone(),
        }
    }

    pub(crate) fn from_params(client_key: &str, params: Params) -> Self {
        Self {
            sender: params.sender,
            action: "sendTG".to_string(),
            client_key: client_key.to_string(),
            telegram_id: params.id,
            secret_key: params.secret_key,
            recipient: params.recipient,
            tg_type: params.tg_type,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum TgType {
    Recruitment,
    Standard,
}

impl Serialize for TgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            TgType::Recruitment => serializer.serialize_str("recruitment"),
            TgType::Standard => serializer.serialize_str("standard"),
        }
    }
}

impl<'de> Deserialize<'de> for TgType {
    fn deserialize<D>(deserializer: D) -> Result<TgType, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        match s.as_str() {
            "recruitment" => Ok(TgType::Recruitment),
            "standard" => Ok(TgType::Standard),
            _ => Err(serde::de::Error::custom("invalid telegram type")),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Params {
    pub(crate) sender: String,
    pub(crate) id: String,
    pub(crate) recipient: String,
    pub(crate) secret_key: String,
    pub(crate) tg_type: TgType,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Header {
    pub(crate) recipient: String,
    pub(crate) telegram_id: String,
}

impl PartialEq for Header {
    fn eq(&self, other: &Self) -> bool {
        self.recipient == other.recipient && self.telegram_id == other.telegram_id
    }
}

#[derive(Debug)]
pub(crate) struct Command {
    pub(crate) operation: Operation,
    pub(crate) tx: oneshot::Sender<Response>,
}

impl Command {
    pub(crate) fn new(action: Operation, tx: oneshot::Sender<Response>) -> Self {
        Self {
            operation: action,
            tx,
        }
    }
}

#[derive(Debug)]
pub(crate) enum Operation {
    Queue(Params),
    Delete(Header),
    List,
}

#[derive(Debug)]
pub(crate) enum Response {
    Ok,
    // Error(Error),
    List(HashMap<String, Vec<response::Telegram>>),
}
