use std::{collections::HashSet, fmt::Display, sync::Arc};

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::{Deserialize, Serialize};

use crate::{domain::auth::save_auth, slack::helpers};

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

#[derive(Serialize)]
pub struct OAuthAccessRequest {
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
}

#[derive(Deserialize)]
pub struct OAuthAccessRawResponse {
    pub token_type: Option<String>,
    pub access_token: Option<String>,
    pub team: Option<OAuthTeamResponse>,
    pub scope: Option<String>,
}

#[derive(Deserialize)]
pub struct OAuthTeamResponse {
    pub id: String,
}

#[derive(Debug)]
pub struct OAuthAccessResponse {
    pub token_type: String,
    pub access_token: String,
    pub team_id: String,
    pub scope: String,
}

impl TryFrom<OAuthAccessRawResponse> for OAuthAccessResponse {
    type Error = hyper::StatusCode;

    fn try_from(value: OAuthAccessRawResponse) -> Result<Self, Self::Error> {
        let result: Result<OAuthAccessResponse, String> = (move || {
            Ok(Self {
                token_type: value.token_type.ok_or("no token type")?,
                access_token: value.access_token.ok_or("no access token")?,
                team_id: value.team.ok_or("no team")?.id,
                scope: value.scope.ok_or("no scope")?,
            })
        })();
        match result {
            Ok(response) => {
                if response.token_type != "bot" {
                    log::error!(
                        "expected oauth access token of bot type but found {}",
                        response.token_type
                    );
                    return Err(Self::Error::FORBIDDEN);
                }
                let scopes: HashSet<String> =
                    response.scope.split(",").map(|v| v.to_string()).collect();
                for scope in vec!["commands", "channels:join", "chat:write"].into_iter() {
                    if !scopes.contains(scope) {
                        log::error!("oauth access does not contain scope {}", scope);
                        return Err(Self::Error::FORBIDDEN);
                    }
                }
                Ok(response)
            }
            Err(err) => {
                log::error!("invalid oauth access response: {}", err);
                Err(Self::Error::FORBIDDEN)
            }
        }
    }
}

pub async fn execute(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OAuthQuery>,
) -> Result<Redirect, hyper::StatusCode> {
    log::trace!("received oauth authorization: {}", query);

    if let Some(..) = query.error {
        return Err(hyper::StatusCode::UNAUTHORIZED);
    } else if let None = query.code {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }

    let request_body = serde_urlencoded::to_string(&OAuthAccessRequest {
        client_id: state.configs.client_id.clone(),
        client_secret: state.configs.client_secret.clone(),
        code: query.code.unwrap(),
    })
    .map_err(|err| {
        log::error!("could not create oauth access request payload: {}", err);
        hyper::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = helpers::send_post_with_type(
        "https://slack.com/api/oauth.v2.access",
        hyper::Body::from(request_body),
        String::from("application/x-www-form-urlencoded"),
    )
    .await
    .map_err(|err| {
        log::error!("unable to send oauth access request: {}", err);
        hyper::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response: OAuthAccessResponse = serde_json::from_str::<OAuthAccessRawResponse>(&response)
        .map_err(|err| {
            log::error!("unable to parse oauth access response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .try_into()?;

    let request = save_auth::Request {
        team: response.team_id.clone(),
        access_token: response.access_token.clone(),
    };
    if let Err(err) = save_auth::execute(state.auth_repo.clone(), request).await {
        log::error!("unable to save oauth access token: {:?}", err);
        return Err(hyper::StatusCode::INTERNAL_SERVER_ERROR);
    }

    log::trace!(
        "saved oauth access token: token_id={}, access_token={}",
        response.team_id,
        response.access_token
    );

    Ok(Redirect::to(&format!(
        "https://slack.com/app_redirect?app={}",
        state.configs.app_id
    )))
}
