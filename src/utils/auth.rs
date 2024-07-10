use crate::core::error::Error;
use crate::core::state::AppState;
use axum::extract::State;
use axum::{body::Body, extract::Request, http, http::Response, middleware::Next};
use chrono::{Duration, Utc};
use jsonwebtoken::{self, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct Claims {
    pub(crate) exp: usize,
    pub(crate) iat: usize,
    pub(crate) nation: String,
}

#[derive(Clone)]
pub(crate) struct User {
    pub(crate) nation: String,
    pub(crate) password_hash: String,
}

pub(crate) fn encode_jwt(nation: String) -> Result<String, Error> {
    let secret = "randomStringTypicallyFromENV".to_string();
    let current_time = Utc::now();
    let expiration_time = current_time + Duration::days(1);

    let exp = expiration_time.timestamp() as usize;
    let iat = current_time.timestamp() as usize;

    let claims = Claims { exp, iat, nation };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(Error::JWTEncode)
}

pub(crate) fn decode_jwt(token: String) -> Result<TokenData<Claims>, Error> {
    let secret = "randomStringTypicallyFromENV".to_string();

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
    let token_data = decode_jwt(token.unwrap().to_string())?;

    // Fetch the user details from the database
    let current_user = match state
        .retrieve_user_by_nation(&token_data.claims.nation)
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return Err(Error::Unauthorized),
        Err(_) => return Err(Error::Placeholder),
    };

    request.extensions_mut().insert(current_user);

    Ok(next.run(request).await)
}
