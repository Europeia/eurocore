use axum::http::StatusCode;
use axum::response::{Response, IntoResponse};
use axum_macros::FromRequest;
use serde::Serialize;

pub enum Error {
    Config(config::ConfigError),
    DatabaseMigration(sqlx::migrate::MigrateError),
    IO(std::io::Error),
    DatabaseConnectionFailure(sqlx::Error),
    ReqwestClientBuild(reqwest::Error),
    HTTPClient(reqwest::Error),
    ExternalServer(reqwest::Error),
    URLEncode(serde_urlencoded::ser::Error),
    HeaderDecode(reqwest::header::ToStrError),
    Deserialize(quick_xml::DeError),
    InvalidFactbookCategory,
    Regex(regex::Error),
    Placeholder,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Config(e) => {
                write!(f, "Config error: {}", e)
            }
            Error::DatabaseMigration(e) => {
                write!(f, "Database migration error: {}", e)
            }
            Error::IO(e) => {
                write!(f, "IO error: {}", e)
            }
            Error::DatabaseConnectionFailure(e) => {
                write!(f, "Database connection error: {}", e)
            }
            Error::ReqwestClientBuild(e) => {
                write!(f, "Reqwest client build error: {}", e)
            }
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
            Error::Regex(e) => {
                write!(f, "Regex error: {}", e)
            }
            Error::Placeholder => {
                write!(f, "No dispatch ID returned")
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
            Error::Config(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Config error")
            }
            Error::DatabaseMigration(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database migration error")
            }
            Error::IO(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "IO error")
            }
            Error::DatabaseConnectionFailure(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database connection failure")
            }
            Error::ReqwestClientBuild(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Reqwest client build error")
            }
            Error::HTTPClient(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Reqwest client internal error")
            }
            Error::ExternalServer(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Server error")
            }
            Error::URLEncode(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "URL encoding error")
            }
            Error::HeaderDecode(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Header decode error")
            }
            Error::Deserialize(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Deserialization error")
            }
            Error::InvalidFactbookCategory => {
                (StatusCode::BAD_REQUEST, "Invalid factbook category")
            }
            Error::Regex(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Regex error")
            }
            Error::Placeholder => {
                (StatusCode::INTERNAL_SERVER_ERROR, "No dispatch ID returned")
            }
        };

        (status, AppJson(ErrorResponse { message: message.to_string() })).into_response()
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