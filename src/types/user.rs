pub(crate) type Username = String;

#[derive(Clone, Debug, sqlx::FromRow)]
pub(crate) struct AuthorizedUser {
    pub(crate) id: i32,
    pub(crate) username: Username,
    pub(crate) password_hash: String,
    pub(crate) claims: Vec<String>,
}
