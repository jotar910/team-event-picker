use async_trait::async_trait;
use axum::{Json, RequestPartsExt};
use axum::{
    response::{IntoResponse, Response},
    TypedHeader,
};
use axum::extract::FromRequestParts;
use axum::headers::Authorization;
use axum::headers::authorization::Bearer;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::middleware::Next;
use hyper::Request;
use jsonwebtoken::{decode, DecodingKey, encode, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::helpers::date::Date;

pub fn generate_jwt_token(
    team_id: String,
    access_token: String,
) -> String {
    let exp = Date::new(Date::now().timestamp() + 24 * 60 * 60).into();
    let claims = Claims {
        team_id,
        access_token,
        exp
    };
    // Create the authorization token
    let token = encode(&Header::default(), &claims, &KEYS.encoding).map_err(
        |_| AuthError::TokenCreation
    ).expect("could not create token");

    return token;
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub team_id: String,
    pub access_token: String,
    pub exp: i64,
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>().await
            .map_err(|_| AuthError::InvalidToken)?;
        // Decode the user data
        let token = bearer.token();
        let token_data = decode::<Claims>(
            token,
            &KEYS.decoding,
            &Validation::default(),
        ).map_err(|err| {
            log::trace!("error decoding token: {:?}", err);
            AuthError::InvalidToken
        })?;

        if token_data.claims.exp < Date::now().timestamp() {
            return Err(AuthError::TokenExpired);
        }

        Ok(token_data.claims)
    }
}

pub async fn test() -> String {
    return "Hello, World!".to_string();
}

pub async fn middleware<B>(claims: Claims, request: Request<B>, next: Next<B>) -> Response {
    log::trace!("authenticated with claims: {:?}", claims);

    next.run(request).await
}

#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    WrongCredentials,
    TokenCreation,
    MissingCredentials,
    TokenExpired,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub static KEYS: Lazy<Keys> = Lazy::new(|| {
    // let secret = Alphanumeric.sample_string(&mut rand::thread_rng(), 60);
    // Keys::new(secret.as_bytes())
    Keys::new("secret".as_bytes())
});

pub struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_jwt_token;

    #[test]
    fn generate_token_example() {
        let token = generate_jwt_token(
            "team_id".to_string(),
            "token".to_string(),
        );
        println!("token: {}", token)
    }
}
