use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
    Extension, RequestPartsExt,
};
use futures::TryStreamExt;
use hyper::StatusCode;
use serde::Deserialize;
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

pub async fn guard(request: Request<Body>, next: Next<Body>) -> Result<Response, StatusCode> {
    let (mut parts, mut body) = request.into_parts();
    let body = response_to_string(&mut body).await?;

    let data: AuthRawRequest = serde_urlencoded::from_str(&body).map_err(|err| {
        log::trace!("failed to deserialize auth raw request: {}: {}", err, body);
        StatusCode::BAD_REQUEST
    })?;
    let data: AuthRequest = data.try_into().map_err(|err| {
        log::trace!("failed to parse auth raw request: {}: {}", err, body);
        StatusCode::BAD_REQUEST
    })?;

    log::trace!("guard request data: {:?}", data);

    let Extension(state) = parts
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
            if let Err(err) = super::send_post(&data.response_url, hyper::Body::from(body)).await {
                log::trace!(
                    "could not send slack response for unauthorized user: {}",
                    err
                );
            }
            return Err(err);
        }
    };

    let mut request = Request::from_parts(parts, Body::from(body));

    let access_token_header: HeaderValue = auth.access_token.parse().map_err(|err| {
        log::error!("could not parse access token: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    request
        .headers_mut()
        .append("x-access-token", access_token_header);

    Ok(next.run(request).await)
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
