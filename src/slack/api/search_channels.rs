use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use client::get_channels as client_get_channels;

use crate::domain::auth::jwt::Claims;
use crate::domain::events::find_all_events;
use crate::repository;
use crate::slack::client;

use super::ApiError;

#[derive(Serialize, Debug)]
pub struct ApiResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub channels: Vec<Channel>,
}

impl ApiResponse {
    fn error(error: String) -> Self {
        Self {
            ok: false,
            error: Some(error),
            channels: vec![],
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub num_events: u32,
}

pub async fn execute(
    claims: Claims,
    event_repo: Arc<dyn repository::event::Repository>,
) -> Result<ApiResponse, ApiError> {
    let response = client_get_channels::new(claims.team_id, claims.access_token)
        .execute()
        .await?;

    if !response.ok {
        return Err(ApiError::InternalServerError(format!(
            "failed to get channels: {:?}",
            response.error
        )));
    }

    let events = find_all_events::execute(
        event_repo.clone(),
        find_all_events::Request::new().with_channels(
            response
                .channels
                .iter()
                .map(|channel| channel.id.clone())
                .collect(),
        ),
    )
        .await
        .inspect_err(|err| log::warn!("failed to get events: {:?}", err))
        .map_err(|err| ApiError::InternalServerError(format!("failed to get events: {:?}", err)))?
        .data;

    let mut events_map: HashMap<String, Vec<&find_all_events::Response>> = HashMap::new();
    for item in events.iter() {
        events_map
            .entry(item.channel.clone())
            .or_insert_with(Vec::new)
            .push(item);
    }

    Ok(ApiResponse {
        ok: true,
        error: None,
        channels: response
            .channels
            .into_iter()
            .map(|channel| Channel {
                id: channel.id.clone(),
                name: channel.name,
                num_events: events_map
                    .get(&channel.id)
                    .map(|events| events.len() as u32)
                    .unwrap_or(0),
            })
            .collect(),
    })
}
