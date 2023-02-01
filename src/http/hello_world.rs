use std::sync::Arc;

use axum::extract::State;

use super::AppState;

// basic handler that responds with a string
pub async fn root(
    State(state): State<Arc<AppState>>
) -> String {
    state.config.database_url.to_owned()
}