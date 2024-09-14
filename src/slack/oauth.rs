use std::{fmt::Display, sync::Arc};

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;

use crate::domain::auth::authenticate;

use super::state::AppState;

#[derive(Deserialize)]
pub struct OAuthQuery {
    pub code: Option<String>,
    pub error: Option<String>,
}

impl Display for OAuthQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(err) = self.error.clone() {
            return write!(f, "error={}", err);
        }
        if let Some(code) = self.code.clone() {
            return write!(f, "code={}", code);
        }
        write!(f, "empty")
    }
}

pub async fn execute_with_redirect(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OAuthQuery>,
) -> Result<Redirect, hyper::StatusCode> {
    authenticate::execute(state.auth_repo.clone(), authenticate::OAuthRequest{
        client_id: state.configs.client_id.clone(),
        client_secret: state.configs.client_secret.clone(),
        code: query.code,
        error: query.error,
    }).await?;

    Ok(Redirect::to(&format!(
        "https://slack.com/app_redirect?app={}",
        state.configs.app_id
    )))
}

pub async fn execute(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OAuthQuery>,
) -> Result<authenticate::OAuthResponse, hyper::StatusCode> {
    authenticate::execute(state.auth_repo.clone(), authenticate::OAuthRequest{
        client_id: state.configs.client_id.clone(),
        client_secret: state.configs.client_secret.clone(),
        code: query.code,
        error: query.error,
    }).await
}
