use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Args {
    pub(crate) user: String,
    pub(crate) database_host: String,
    pub(crate) database_port: u16,
    pub(crate) database_name: String,
    pub(crate) database_user: String,
    pub(crate) database_password: String,
    pub(crate) log_level: String,
    pub(crate) port: u16,
    pub(crate) nations: String,
    pub(crate) secret: String,
    pub(crate) telegram_client_key: String,
}

pub(crate) fn create_nations_map(nations: &str) -> std::collections::HashMap<String, String> {
    nations
        .split(',')
        .map(|nation| {
            let mut split = nation.split(':');
            let key = split.next().unwrap().to_string();
            let value = split.next().unwrap().to_string();
            (key, value)
        })
        .collect()
}
