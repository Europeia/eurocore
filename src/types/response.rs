use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct DispatchHeader {
    pub(crate) id: i32,
    pub(crate) nation: String,
}

#[derive(Serialize)]
pub(crate) struct Dispatch {
    pub(crate) id: i32,
    pub(crate) nation: String,
    pub(crate) category: i16,
    pub(crate) subcategory: i16,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) created_by: String,
    pub(crate) modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub(crate) struct DispatchStatus {
    pub(crate) id: i32,
    pub(crate) action: String,
    pub(crate) status: String,
    pub(crate) dispatch_id: Option<i32>,
    pub(crate) error: Option<String>,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
    pub(crate) modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub(crate) struct RmbPostStatus {
    pub(crate) id: i32,
    pub(crate) status: String,
    pub(crate) rmbpost_id: Option<i32>,
    pub(crate) error: Option<String>,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
    pub(crate) modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Debug)]
pub(crate) struct Telegram {
    recipient: String,
    id: String,
}

impl Telegram {
    pub(crate) fn new(recipient: &str, telegram_id: &str) -> Self {
        Self {
            recipient: recipient.to_string(),
            id: telegram_id.to_string(),
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct Login {
    username: String,
    token: String,
}

impl Login {
    pub(crate) fn new(username: &str, token: &str) -> Self {
        Self {
            username: username.to_string(),
            token: token.to_string(),
        }
    }
}
