use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use crate::domain::auth::jwt::Claims;
use crate::slack::api::{search_channels, search_events};
use crate::slack::api::core::ApiError;
use crate::slack::state::AppState;

pub async fn search_channels(
   claims: Claims,
    State(state): State<Arc<AppState>>
) -> Result<search_channels::ApiResponse, ApiError> {
   search_channels::execute(claims, state.event_repo.clone()).await.into()
}

pub async fn search_events(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<search_events::ApiRequest>,
) -> Result<search_events::ApiResponse, ApiError> {
    search_events::execute(claims, state.event_repo.clone(), payload).await.into()
}

// get event details

// pick participant for some event

// repick participant for some event

// cancel pick of participant for some event

// unpick participant for some event
