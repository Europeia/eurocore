use crate::core::error::Error;
use crate::core::state::AppState;
use axum::extract::State;
use axum::{body::Body, extract::Request, http, http::Response, middleware::Next};
use chrono::{Duration, Utc};
use jsonwebtoken::{self, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct Claims {
    pub(crate) exp: usize,
    pub(crate) iat: usize,
    pub(crate) sub: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub(crate) struct User {
    pub(crate) username: String,
    pub(crate) password_hash: String,
    pub(crate) claims: sqlx::types::Json<Vec<String>>,
}

pub(crate) fn encode_jwt(nation: String, secret: &str) -> Result<String, Error> {
    let current_time = Utc::now();
    let expiration_time = current_time + Duration::days(1);

    let exp = expiration_time.timestamp() as usize;
    let iat = current_time.timestamp() as usize;

    let claims = Claims {
        exp,
        iat,
        sub: nation,
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(Error::JWTEncode)
}

pub(crate) fn decode_jwt(token: String, secret: &str) -> Result<TokenData<Claims>, Error> {
    jsonwebtoken::decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map_err(Error::JWTDecode)
}

pub(crate) async fn authorize(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>, Error> {
    let auth_header = request.headers_mut().get(http::header::AUTHORIZATION);
    let auth_header = match auth_header {
        Some(header) => header.to_str().map_err(Error::HeaderDecode)?,
        None => return Err(Error::NoJWT),
    };
    let mut header = auth_header.split_whitespace();
    let (bearer, token) = (header.next(), header.next());
    let token_data = decode_jwt(token.unwrap().to_string(), &state.secret)?;

    let current_user = match state
        .retrieve_user_by_username(&token_data.claims.sub)
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return Err(Error::Unauthorized),
        Err(e) => return Err(e),
    };

    let path = request.uri().path();
    let method = request.method().as_str();

    if let Some(permission) = get_required_permission(path, method) {
        if !current_user.claims.contains(&permission) {
            return Err(Error::Unauthorized);
        }
    }

    request.extensions_mut().insert(current_user);

    Ok(next.run(request).await)
}

fn get_required_permission(path: &str, method: &str) -> Option<String> {
    match path {
        "/dispatch" => match method {
            "POST" => Some("dispatches.create".into()),
            "PUT" => Some("dispatches.edit".into()),
            "DELETE" => Some("dispatches.delete".into()),
            _ => None,
        },
        _ => None,
    }
}
