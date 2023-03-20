use axum::{
    body::Body,
    http::{request::Parts, HeaderValue, Request},
    middleware::Next,
    response::Response,
    Extension, RequestPartsExt,
};
use chrono::Utc;
use futures::TryStreamExt;
use hmac::{Hmac, Mac};
use hyper::{HeaderMap, StatusCode};
use serde::Deserialize;
use sha2::Sha256;
use std::{fmt::Debug, sync::Arc};

use crate::domain::verify_auth;

use super::state::AppState;

#[derive(Debug, Deserialize)]
struct AuthRequest {
    pub team_id: String,
    pub response_url: String,
}

#[derive(Deserialize)]
struct AuthRawRequest {
    pub team_id: Option<String>,
    pub response_url: Option<String>,
    pub payload: Option<String>,
}

#[derive(Deserialize)]
struct AuthPayloadRequest {
    pub user: AuthUserRequest,
    pub response_url: String,
}

#[derive(Deserialize)]
struct AuthUserRequest {
    pub team_id: String,
}

impl TryFrom<AuthRawRequest> for AuthRequest {
    type Error = String;

    fn try_from(value: AuthRawRequest) -> Result<Self, Self::Error> {
        if let Some(payload) = value.payload {
            let data = match serde_json::from_str::<AuthPayloadRequest>(&payload) {
                Ok(payload) => Self {
                    team_id: payload.user.team_id,
                    response_url: payload.response_url,
                },
                Err(err) => return Err(err.to_string()),
            };
            return Ok(data);
        }
        Ok(AuthRequest {
            team_id: value.team_id.ok_or("no team_id")?,
            response_url: value.response_url.ok_or("no response_url")?,
        })
    }
}

struct Guard {
    parts: Parts,
    body: String,
    headers: HeaderMap,
    state: Arc<AppState>,
}

impl Guard {
    async fn new(request: Request<Body>) -> Result<Self, StatusCode> {
        let (mut parts, mut body) = request.into_parts();
        let headers = parts.headers.clone();
        let body = response_to_string(&mut body).await?;

        let Extension(state) =
            parts
                .extract::<Extension<Arc<AppState>>>()
                .await
                .map_err(|err| {
                    log::error!("could not find app state on request: {}", err);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

        Ok(Self {
            parts,
            body,
            headers,
            state,
        })
    }

    async fn validate_signature(&self) -> Result<(), StatusCode> {
        if !self.headers.contains_key("x-slack-request-timestamp")
            || !self.headers.contains_key("x-slack-signature")
        {
            log::trace!("unable to find authentication headers");
            return Err(StatusCode::BAD_REQUEST);
        }

        let timestamp: i64 = self
            .headers
            .get("x-slack-request-timestamp")
            .unwrap()
            .to_str()
            .unwrap_or("")
            .parse()
            .unwrap_or(0);

        // verify that the timestamp does not differ from local time by more than five minutes
        if (Utc::now().timestamp() - timestamp).abs() > 300 {
            log::trace!("request is too old");
            return Err(StatusCode::UNAUTHORIZED);
        }

        let base_str = format!("v0:{}:{}", timestamp, self.body);

        let expected_signature = calculate_signature(&base_str, &self.state.secret);

        let received_signature: String = self
            .headers
            .get("x-slack-signature")
            .unwrap()
            .to_str()
            .unwrap_or("")
            .to_string();

        // match the two signatures
        if expected_signature != received_signature {
            log::trace!("signature mismatch");
            return Err(StatusCode::UNAUTHORIZED);
        }

        log::debug!("signature verified");
        Ok(())
    }

    async fn validate_token(&mut self) -> Result<(), StatusCode> {
        let data: AuthRawRequest = serde_urlencoded::from_str(&self.body).map_err(|err| {
            log::trace!(
                "failed to deserialize auth raw request: {}: {}",
                err,
                self.body
            );
            StatusCode::BAD_REQUEST
        })?;
        let data: AuthRequest = data.try_into().map_err(|err| {
            log::trace!("failed to parse auth raw request: {}: {}", err, self.body);
            StatusCode::BAD_REQUEST
        })?;

        log::trace!("guard request data: {:?}", data);

        let Extension(state) = self
            .parts
            .extract::<Extension<Arc<AppState>>>()
            .await
            .map_err(|err| {
                log::error!("could not find app state on request: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let auth = match verify_auth::execute(
            state.auth_repo.clone(),
            verify_auth::Request {
                team: data.team_id.clone(),
            },
        )
        .await
        {
            Ok(auth) => {
                log::trace!("auth verification with success: {}", auth);
                auth
            }
            Err(err) => {
                log::trace!(
                    "auth verification failed for team {}: {:?}",
                    data.team_id,
                    err
                );
                let err = match err {
                    verify_auth::Error::Unauthorized => StatusCode::UNAUTHORIZED,
                    verify_auth::Error::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                };
                let body = super::to_response_error(&format!(
                    "Error {}: {}.",
                    err.as_str(),
                    err.canonical_reason().unwrap_or("Unknown")
                ))?;
                if let Err(err) =
                    super::send_post(&data.response_url, hyper::Body::from(body)).await
                {
                    log::trace!(
                        "could not send slack response for unauthorized user: {}",
                        err
                    );
                }
                return Err(err);
            }
        };

        let access_token_header: HeaderValue = auth.access_token.parse().map_err(|err| {
            log::error!("could not parse access token: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        self.headers.append("x-access-token", access_token_header);
        
        log::debug!("user authenticated");
        Ok(())
    }

    fn request(self) -> Request<Body> {
        let mut request = Request::from_parts(self.parts, Body::from(self.body));
        request.headers_mut().extend(self.headers);
        request
    }
}

pub async fn validate(request: Request<Body>, next: Next<Body>) -> Result<Response, StatusCode> {
    let mut guard = Guard::new(request).await?;
    log::trace!("auth guard: validating signature");
    guard.validate_signature().await?;
    log::trace!("auth guard: validating token");
    guard.validate_token().await?;
    Ok(next.run(guard.request()).await)
}

async fn response_to_string(stream: &mut Body) -> Result<String, StatusCode> {
    let entire_body = stream
        .try_fold(Vec::new(), |mut data, chunk| async move {
            data.extend_from_slice(&chunk);
            Ok(data)
        })
        .await
        .map_err(|err| {
            log::error!("could not read from body stream: {}", err);
            StatusCode::BAD_REQUEST
        })?;
    let entire_body = String::from_utf8(entire_body).map_err(|err| {
        log::error!("response was not valid utf-8: {}", err);
        StatusCode::BAD_REQUEST
    })?;
    Ok(entire_body)
}

fn calculate_signature(base_str: &str, secret: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(base_str.as_bytes());
    let result = mac.finalize().into_bytes();
    format!("v0={}", hex::encode(result))
}
