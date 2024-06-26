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

use crate::domain::auth::verify_auth;
use crate::domain::events::find_all_events;

use super::state::AppState;

#[derive(Debug, Deserialize)]
struct RequestData {
    pub team_id: String,
    pub response_url: String,
    pub channel_id: String,
    pub actions: Vec<String>,
}

#[derive(Deserialize)]
struct InboundRequest {
    pub team_id: Option<String>,
    pub channel_id: Option<String>,
    pub response_url: Option<String>,
    pub text: Option<String>,
    pub payload: Option<String>,
}

#[derive(Deserialize)]
struct InboundRequestPayload {
    pub response_url: String,
    pub channel: InboundRequestChannel,
    pub user: InboundRequestUser,
    pub actions: Vec<InboundRequestAction>,
}

#[derive(Deserialize)]
struct InboundRequestUser {
    pub team_id: String,
}

#[derive(Deserialize)]
struct InboundRequestChannel {
    pub id: String,
}

#[derive(Deserialize)]
struct InboundRequestAction {
    pub block_id: Option<String>,
}

impl TryFrom<InboundRequest> for RequestData {
    type Error = String;

    fn try_from(value: InboundRequest) -> Result<Self, Self::Error> {
        if let Some(payload) = value.payload {
            let data = match serde_json::from_str::<InboundRequestPayload>(&payload) {
                Ok(payload) => Self {
                    team_id: payload.user.team_id,
                    channel_id: payload.channel.id,
                    actions: payload
                        .actions
                        .into_iter()
                        .map(|action| action.block_id)
                        .filter(|action| action.is_some())
                        .map(|block_id| block_id.unwrap())
                        .collect(),
                    response_url: payload.response_url,
                },
                Err(err) => return Err(err.to_string()),
            };
            return Ok(data);
        }
        Ok(RequestData {
            team_id: value.team_id.ok_or("no team_id")?,
            channel_id: value.channel_id.ok_or("no channel_id")?,
            actions: vec![value.text.ok_or("no command text")?],
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
        let slack_request_timestamp = self.headers.get("x-slack-request-timestamp");
        let slack_signature = self.headers.get("x-slack-signature");
        log::debug!(
            "verifying signature: x-slack-request-timestamp={:?},x-slack-signature={:?}",
            slack_request_timestamp,
            slack_signature
        );

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

        let expected_signature = calculate_signature(&base_str, &self.state.configs.secret);

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
        let data = self.data()?;

        log::trace!("guard request data: {:?}", data);

        let auth = match verify_auth::execute(
            self.state.auth_repo.clone(),
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
                return Guard::send_error(
                    &data.response_url,
                    match err {
                        verify_auth::Error::Unauthorized => StatusCode::UNAUTHORIZED,
                        verify_auth::Error::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                    },
                )
                .await;
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

    async fn validate_plan(&mut self) -> Result<(), StatusCode> {
        let data = self.data()?;

        let events = match find_all_events::execute(
            self.state.event_repo.clone(),
            find_all_events::Request {
                channel: data.channel_id.clone(),
            },
        )
        .await
        {
            Ok(list) => {
                log::trace!(
                    "found {} events on channel {}",
                    list.data.len(),
                    data.channel_id
                );
                list.data
            }
            Err(err) => {
                log::trace!(
                    "could not verify total events on channel {} for team {}: {:?}",
                    data.channel_id,
                    data.team_id,
                    err
                );
                return Guard::send_error(
                    &data.response_url,
                    match err {
                        find_all_events::Error::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                    },
                )
                .await;
            }
        };

        let reached_limit = events.len() > 0;
        if reached_limit
            && (data.actions.contains(&String::from("create"))
                || data.actions.contains(&String::from("add_event_actions")))
        {
            log::trace!(
                "cannot create more events on channel {} for team {} (current={})",
                data.channel_id,
                data.team_id,
                events.len()
            );
            return Guard::send_error(&data.response_url, StatusCode::FORBIDDEN).await;
        }

        let reached_limit_header: HeaderValue =
            reached_limit.to_string().parse().map_err(|err| {
                log::error!("could not parse reached limit state: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        self.headers.append("x-reached-limit", reached_limit_header);

        log::trace!("plan validated for {:?}", data.actions);

        Ok(())
    }

    fn data(&self) -> Result<RequestData, StatusCode> {
        let data: InboundRequest = serde_urlencoded::from_str(&self.body).map_err(|err| {
            log::trace!(
                "failed to deserialize auth raw request: {}: {}",
                err,
                self.body
            );
            StatusCode::BAD_REQUEST
        })?;
        data.try_into().map_err(|err| {
            log::trace!("failed to parse auth raw request: {}: {}", err, self.body);
            StatusCode::BAD_REQUEST
        })
    }

    fn request(self) -> Request<Body> {
        let mut request = Request::from_parts(self.parts, Body::from(self.body));
        request.headers_mut().extend(self.headers);
        request
    }

    async fn send_error(response_url: &str, err: StatusCode) -> Result<(), StatusCode> {
        let body = super::to_response_error(&format!(
            "Error {}: {}.",
            err.as_str(),
            err.canonical_reason().unwrap_or("Unknown")
        ))?;
        if let Err(err) = super::send_post(response_url, hyper::Body::from(body)).await {
            log::trace!(
                "could not send slack response for unauthorized user: {}",
                err
            );
        }
        Err(err)
    }
}

pub async fn validate(request: Request<Body>, next: Next<Body>) -> Result<Response, StatusCode> {
    let mut guard = Guard::new(request).await?;
    log::trace!("auth guard: validating signature");
    guard.validate_signature().await?;
    log::trace!("auth guard: validating token");
    guard.validate_token().await?;
    log::trace!("auth guard: validating team plan");
    guard.validate_plan().await?;
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
