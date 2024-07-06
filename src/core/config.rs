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
    pub(crate) ns_nation: String,
    pub(crate) ns_password: String,
}