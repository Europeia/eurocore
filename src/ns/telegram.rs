use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Telegram {
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
}

impl std::fmt::Display for Telegram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.telegram_id, self.recipient)
    }
}

impl Telegram {
    pub(crate) fn from_params(client_key: &str, params: TelegramParams) -> Self {
        Self {
            action: "sendTG".to_string(),
            client_key: client_key.to_string(),
            recipient: params.recipient.to_lowercase().replace(" ", "_"),
            telegram_id: params.telegram_id,
            secret_key: params.secret_key,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct TelegramParams {
    pub(crate) recipient: String,
    pub(crate) telegram_id: String,
    pub(crate) secret_key: String,
    pub(crate) recruitment: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TelegramHeader {
    pub(crate) recipient: String,
    pub(crate) telegram_id: String,
}
