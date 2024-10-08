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
