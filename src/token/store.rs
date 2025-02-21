use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

pub(crate) struct Token {
    username: String,
    expiry: Instant,
    attributes: Arc<Mutex<HashMap<String, String>>>,
}

impl Token {
    pub(crate) fn new(username: String, expiry: Instant) -> Self {
        Self {
            username,
            expiry,
            attributes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub(crate) trait TokenStore {
    async fn create(&self, token: Token) -> String;

    async fn read(&self, token_id: String) -> Option<Token>;
}
