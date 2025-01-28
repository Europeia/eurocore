use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct UpdatePasswordData {
    pub(crate) new_password: String,
}
