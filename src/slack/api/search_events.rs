use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use client::get_users as client_get_users;

use crate::domain::auth::jwt::Claims;
use crate::domain::events::find_all_events;
use crate::repository;
use crate::slack::client;

use super::ApiError;

#[derive(Deserialize, Debug)]
pub struct ApiRequest {
    pub channel: String,
}

#[derive(Serialize, Debug)]
pub struct ApiResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub events: Vec<Event>,
}

impl ApiResponse {
    fn error(error: String) -> Self {
        Self {
            ok: false,
            error: Some(error),
            events: vec![],
        }
    }
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        let body = serde_json::to_string(&self)
            .inspect_err(|err| log::warn!("failed to serialize response body: {:?}", err))
            .unwrap_or(
                serde_json::to_string(&ApiResponse::error(
                    "failed to serialize body".to_string(),
                ))
                    .expect("failed to serialize error response"),
            );
        (StatusCode::OK, body).into_response()
    }
}

#[derive(Serialize, Debug)]
pub struct Event {
    pub id: u32,
    pub name: String,
    pub users: Vec<Participant>,
}

#[derive(Serialize, Debug)]
pub struct Participant {
    pub id: String,
    pub username: String,
    pub name: String,
    pub picked: bool,
    pub picked_at: Option<i64>,
}

pub async fn execute(
    claims: Claims,
    event_repo: Arc<dyn repository::event::Repository>,
    request: ApiRequest,
) -> Result<ApiResponse, ApiError> {
    let events = find_all_events::execute(
        event_repo.clone(),
        find_all_events::Request::new().with_channel(request.channel),
    )
        .await
        .inspect_err(|err| log::warn!("failed to get events: {:?}", err))
        .map_err(|err| ApiError::InternalServerError(format!("failed to get events: {:?}", err)))?
        .data;

    if events.is_empty() {
        return Ok(ApiResponse {
            ok: true,
            error: None,
            events: vec![],
        });
    }

    let response = client_get_users::new(claims.team_id, claims.access_token)
        .execute()
        .await?;

    if !response.ok {
        return Err(ApiError::InternalServerError(format!(
            "failed to get channels: {:?}",
            response.error
        )));
    }

    let users: HashMap<_, _> = response
        .members
        .into_iter()
        .map(|user| (user.id.clone(), user))
        .collect();

    Ok(ApiResponse {
        ok: true,
        error: None,
        events: events
            .into_iter()
            .map(|event| {
                Event {
                    id: event.id,
                    name: event.name,
                    users: event
                        .participants
                        .into_iter()
                        .map(|participant| {
                            let binding = client_get_users::ClientUser {
                                id: participant.user.clone(),
                                name: participant.user.clone(),
                                real_name: participant.user.clone(),
                            };
                            let user = users.get(&participant.user).unwrap_or(&binding);
                            Participant {
                                id: user.id.clone(),
                                username: user.name.clone(),
                                name: user.real_name.clone(),
                                picked: participant.picked,
                                picked_at: participant.picked_at,
                            }
                        })
                        .collect(),
                }
            })
            .collect(),
    })
}
