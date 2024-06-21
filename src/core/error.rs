pub enum Error {
    DatabaseConnectionFailure(sqlx::Error),
    ReqwestClientBuildError(reqwest::Error),
    PoisonedLock
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DatabaseConnectionFailure(e) => {
                write!(f, "Database connection error: {}", e)
            },
            Error::ReqwestClientBuildError(e) => {
                write!(f, "Reqwest client build error: {}", e)
            },
            Error::PoisonedLock => {
                write!(f, "Lock poisoned")
            },
        }
    }
}