use crate::core::error::{self, Error};
use crate::core::state::AppState;
use crate::types::response::User;
use crate::types::{response, AuthorizedUser, Username};
use crate::utils::auth::{decode_jwt, Claims};
use axum::body::Body;
use axum::extract::{Json, Request, State};
use axum::http::{header, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use regex::Regex;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub(crate) struct UserController {
    pool: PgPool,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    username_pattern: Regex,
}

impl std::fmt::Debug for UserController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserController")
            .field("username_pattern", &self.username_pattern.as_str())
            .finish()
    }
}

impl UserController {
    pub(crate) fn new(pool: PgPool, jwt_secret: String) -> Result<Self, error::ConfigError> {
        Ok(Self {
            pool,
            encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(jwt_secret.as_bytes()),
            username_pattern: Regex::new(r"^[a-zA-Z0-9_-]{3,20}$")?,
        })
    }

    pub(crate) async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<AuthorizedUser>, Error> {
        match sqlx::query(
            "SELECT
            users.id,
            users.username,
            users.password_hash,
            COALESCE(array_agg(permissions.name), '{}') AS permissions
            FROM
                users
            LEFT JOIN
                user_permissions ON users.id = user_permissions.user_id
            LEFT JOIN
                permissions ON user_permissions.permission_id = permissions.id
            WHERE
                users.username = $1
            GROUP BY
                users.id, users.username;",
        )
        .bind(username)
        .map(map_user)
        .fetch_one(&self.pool)
        .await
        {
            Ok(user) => Ok(Some(user)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(Error::Sql(e)),
        }
    }

    pub(crate) async fn get_username_by_id(&self, id: i32) -> Result<Option<Username>, Error> {
        match sqlx::query("SELECT username FROM users WHERE id = $1")
            .bind(id)
            .map(|row: PgRow| row.get("username"))
            .fetch_one(&self.pool)
            .await
        {
            Ok(username) => Ok(Some(username)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(Error::Sql(e)),
        }
    }

    pub(crate) async fn register(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(AuthorizedUser, String), Error> {
        if !self.username_pattern.is_match(username) {
            return Err(Error::InvalidUsername);
        }

        if password.len() < 8 {
            return Err(Error::InvalidPassword(
                "Password must be at least 8 characters".to_owned(),
            ));
        }

        let password_hash = self.hash(&password)?;

        let id: i32 = match sqlx::query(
            "INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id;",
        )
        .bind(username)
        .bind(&password_hash)
        .map(|row: PgRow| row.get("id"))
        .fetch_one(&self.pool)
        .await
        {
            Ok(id) => id,
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                return Err(Error::UserAlreadyExists)
            }
            Err(e) => return Err(Error::Sql(e)),
        };

        let user = AuthorizedUser {
            id,
            username: username.into(),
            password_hash,
            claims: Vec::new(),
        };

        let token = self.encode_jwt(&user)?;

        Ok((user, token))
    }

    pub(crate) async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(AuthorizedUser, String), Error> {
        let user = self
            .get_user_by_username(username)
            .await?
            .ok_or(Error::InvalidUsername)?;

        if let false = bcrypt::verify(password, &user.password_hash)? {
            return Err(Error::Unauthorized);
        };

        let token = self.encode_jwt(&user)?;

        Ok((user, token))
    }

    fn hash(&self, value: &str) -> Result<String, Error> {
        bcrypt::hash(&value, 12).map_err(Error::Bcrypt)
    }

    pub(crate) fn encode_jwt(&self, user: &AuthorizedUser) -> Result<String, Error> {
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
            &self.encoding_key,
        )?)
    }

    pub(crate) fn decode_jwt(&self, token: String) -> Result<TokenData<Claims>, Error> {
        match jsonwebtoken::decode::<Claims>(&token, &self.decoding_key, &Validation::default()) {
            Ok(token_data) => Ok(token_data),
            Err(e) => match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => Err(Error::ExpiredJWT),
                _ => Err(Error::Jwt(e)),
            },
        }
    }
}

pub(crate) async fn authenticate(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>, Error> {
    let headers = request.headers_mut();

    let auth_header = match headers.get(header::AUTHORIZATION) {
        Some(auth_header) => auth_header,
        None => {
            request.extensions_mut().insert(None::<AuthorizedUser>);
            return Ok(next.run(request).await);
        }
    };

    let mut header = auth_header.to_str()?.split_whitespace();

    let (_bearer, token) = (header.next(), header.next().unwrap_or_default());

    let token_data = decode_jwt(token.into(), &state.secret)?;

    let user = state
        .user_controller
        .get_user_by_username(&token_data.claims.sub)
        .await?
        .ok_or_else(|| Error::InvalidUsername)?;

    request.extensions_mut().insert(Some(user));

    Ok(next.run(request).await)
}

fn map_user(row: PgRow) -> AuthorizedUser {
    AuthorizedUser {
        id: row.get("id"),
        username: row.get("username"),
        password_hash: row.get("password_hash"),
        claims: row.get("permissions"),
    }
}
