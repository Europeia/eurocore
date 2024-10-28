use axum::extract::State;
use axum::{body::Body, extract::Request, http, http::Response, middleware::Next};
use chrono::{Duration, Utc};
use jsonwebtoken::{self, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

use crate::core::error::Error;
use crate::core::state::AppState;

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct Claims {
    pub(crate) exp: usize,
    pub(crate) iat: usize,
    pub(crate) sub: String,
    pub(crate) iss: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub(crate) struct User {
    pub(crate) username: String,
    pub(crate) password_hash: String,
    pub(crate) claims: sqlx::types::Json<Vec<String>>,
}

pub(crate) fn encode_jwt(user: &User, secret: &str) -> Result<String, Error> {
    let current_time = Utc::now();
    let expiration_time = current_time + Duration::days(1);

    let exp = expiration_time.timestamp() as usize;
    let iat = current_time.timestamp() as usize;

    let claims = Claims {
        exp,
        iat,
        sub: user.username.to_string(),
        iss: "https://api.europeia.dev".into(),
    };

    Ok(jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )?)
}

pub(crate) fn decode_jwt(token: String, secret: &str) -> Result<TokenData<Claims>, Error> {
    match jsonwebtoken::decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(token_data) => Ok(token_data),
        Err(e) => match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => Err(Error::ExpiredJWT),
            _ => Err(Error::Jwt(e)),
        },
    }
}

pub(crate) async fn authorize(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>, Error> {
    let headers = request.headers_mut();

    let auth_header = headers.get(http::header::AUTHORIZATION);
    let api_key = headers.get("X-API-KEY");

    if auth_header.is_none() && api_key.is_none() {
        return Err(Error::NoCredentials);
    }

    let user = match auth_header {
        Some(header) => {
            let mut header = header.to_str()?.split_whitespace();
            let (_bearer, token) = (header.next(), header.next());

            let token_data = decode_jwt(token.unwrap_or_default().to_string(), &state.secret)?;

            match state
                .retrieve_user_by_username(&token_data.claims.sub)
                .await?
            {
                Some(user) => user,
                None => return Err(Error::Unauthorized),
            }
        }
        None => {
            let api_key = api_key.unwrap().to_str()?;

            match state.retrieve_user_by_api_key(api_key).await? {
                Some(user) => user,
                None => return Err(Error::Unauthorized),
            }
        }
    };

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
