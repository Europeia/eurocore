use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_macros::FromRequest;
use serde::Serialize;
use std::num::ParseIntError;

#[derive(Debug)]
pub enum ConfigError {
    Config(config::ConfigError),
    DatabaseMigration(sqlx::migrate::MigrateError),
    IO(std::io::Error),
    DatabaseConnectionFailure(sqlx::Error),
    ReqwestClientBuild(reqwest::Error),
    Regex(regex::Error),
}

pub enum Error {
    HTTPClient(reqwest::Error),
    ExternalServer(reqwest::Error),
    URLEncode(serde_urlencoded::ser::Error),
    HeaderDecode(reqwest::header::ToStrError),
    Deserialize(quick_xml::DeError),
    InvalidFactbookCategory,
    ParseInt(ParseIntError),
    SQL(sqlx::Error),
    Placeholder,
    DispatchNotFound,
    JWTEncode(jsonwebtoken::errors::Error),
    JWTDecode(jsonwebtoken::errors::Error),
    NoJWT,
    Unauthorized,
    UserAlreadyExists,
    Bcrypt(bcrypt::BcryptError),
    Serialize(serde_json::Error),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::HTTPClient(e) => {
                write!(f, "Reqwest client internal error: {}", e)
            }
            Error::ExternalServer(e) => {
                write!(f, "Server error: {}", e)
            }
            Error::URLEncode(e) => {
                write!(f, "URL encoding error: {}", e)
            }
            Error::HeaderDecode(e) => {
                write!(f, "Header decode error: {}", e)
            }
            Error::Deserialize(e) => {
                write!(f, "Deserialization error: {}", e)
            }
            Error::InvalidFactbookCategory => {
                write!(f, "Invalid factbook category")
            }
            Error::ParseInt(e) => {
                write!(f, "Parse int error: {}", e)
            }
            Error::SQL(e) => {
                write!(f, "SQL error: {}", e)
            }
            Error::Placeholder => {
                write!(f, "No dispatch ID returned")
            }
            Error::DispatchNotFound => {
                write!(f, "Dispatch not found")
            }
            Error::JWTEncode(e) => {
                write!(f, "JWT encode error: {}", e)
            }
            Error::JWTDecode(e) => {
                write!(f, "JWT decode error: {}", e)
            }
            Error::NoJWT => {
                write!(f, "No JWT provided")
            }
            Error::Unauthorized => {
                write!(f, "Unauthorized")
            }
            Error::UserAlreadyExists => {
                write!(f, "User already exists")
            }
            Error::Bcrypt(e) => {
                write!(f, "Bcrypt error: {}", e)
            }
            Error::Serialize(e) => {
                write!(f, "Serialization error: {}", e)
            }
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        tracing::error!("{:?}", self);

        let (status, message) = match self {
            Error::HTTPClient(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Reqwest client internal error",
            ),
            Error::ExternalServer(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Server error"),
            Error::URLEncode(_) => (StatusCode::INTERNAL_SERVER_ERROR, "URL encoding error"),
            Error::HeaderDecode(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Header decode error"),
            Error::Deserialize(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Deserialization error"),
            Error::InvalidFactbookCategory => {
                (StatusCode::BAD_REQUEST, "Invalid factbook category")
            }
            Error::ParseInt(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Parse int error"),
            Error::SQL(_) => (StatusCode::INTERNAL_SERVER_ERROR, "SQL error"),
            Error::Placeholder => (StatusCode::INTERNAL_SERVER_ERROR, "???"),
            Error::DispatchNotFound => (StatusCode::NOT_FOUND, "Dispatch not found"),
            Error::JWTEncode(_) => (StatusCode::INTERNAL_SERVER_ERROR, "JWT encode error"),
            Error::JWTDecode(_) => (StatusCode::INTERNAL_SERVER_ERROR, "JWT decode error"),
            Error::NoJWT => (StatusCode::UNAUTHORIZED, "No JWT provided"),
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            Error::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists"),
            Error::Bcrypt(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Bcrypt error"),
            Error::Serialize(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error"),
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
