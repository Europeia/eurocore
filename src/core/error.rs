use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_macros::FromRequest;
use serde::Serialize;
use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Database migration error: {0}")]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("SQL error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("Reqwest error: {0}")]
    HTTPClient(#[from] reqwest::Error),
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Reqwest error: {0}")]
    HTTPClient(#[from] reqwest::Error),
    #[error("URL encoding error: {0}")]
    URLEncode(#[from] serde_urlencoded::ser::Error),
    #[error("Header decode error: {0}")]
    HeaderDecode(#[from] reqwest::header::ToStrError),
    #[error("Deserialization error: {0}")]
    Deserialize(#[from] quick_xml::DeError),
    #[error("Invalid factbook category")]
    InvalidFactbookCategory,
    #[error("Parse int error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("SQL error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("???")]
    Placeholder,
    #[error("Dispatch not found")]
    DispatchNotFound,
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("No credentials provided")]
    NoCredentials,
    #[error("Expired JWT")]
    ExpiredJWT,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Bcrypt error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Invalid nation")]
    InvalidNation,
    #[error("Internal server error")]
    Internal,
    #[error("Job not found")]
    JobNotFound,
    #[error("Invalid header value: {0}")]
    Header(#[from] axum::http::header::InvalidHeaderValue),
    #[error("Invalid username")]
    InvalidUsername,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        tracing::error!("{:?}", self);

        let (status, message) = match self {
            Error::HTTPClient(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Reqwest error"),
            Error::URLEncode(_) => (StatusCode::INTERNAL_SERVER_ERROR, "URL encoding error"),
            Error::HeaderDecode(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Header decode error"),
            Error::Deserialize(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Deserialization error"),
            Error::InvalidFactbookCategory => {
                (StatusCode::BAD_REQUEST, "Invalid factbook category")
            }
            Error::ParseInt(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Parse int error"),
            Error::Sql(_) => (StatusCode::INTERNAL_SERVER_ERROR, "SQL error"),
            Error::Placeholder => (StatusCode::INTERNAL_SERVER_ERROR, "???"),
            Error::DispatchNotFound => (StatusCode::NOT_FOUND, "Dispatch not found"),
            Error::Jwt(_) => (StatusCode::INTERNAL_SERVER_ERROR, "JWT error"),
            Error::NoCredentials => (StatusCode::UNAUTHORIZED, "No credentials provided"),
            Error::ExpiredJWT => (StatusCode::UNAUTHORIZED, "Expired JWT"),
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            Error::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists"),
            Error::Bcrypt(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Bcrypt error"),
            Error::Serialize(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error"),
            Error::InvalidNation => (StatusCode::BAD_REQUEST, "Invalid nation"),
            Error::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            Error::JobNotFound => (StatusCode::NOT_FOUND, "Job not found"),
            Error::Header(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Invalid header value"),
            Error::InvalidUsername => (StatusCode::BAD_REQUEST, "Invalid username"),
        };

        (
            status,
            AppJson(ErrorResponse {
                message: message.to_string(),
            }),
        )
            .into_response()
    }
}

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(Error))]
struct AppJson<T>(T);

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}
