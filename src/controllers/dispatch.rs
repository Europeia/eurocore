use crate::core::error::Error;
use crate::types::response;
use sqlx::PgPool;

#[derive(Clone)]
pub(crate) struct DispatchController {
    pool: PgPool,
}

impl DispatchController {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(crate) fn queue(&self, action: &str) -> Result<response::DispatchStatus, Error> {}
}
