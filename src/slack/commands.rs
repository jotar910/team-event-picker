use std::sync::Arc;

use axum::{extract::State, Json};
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    domain::pick_participant,
    repository::event::Repository,
};

use super::{templates, AppState};

/// Slack command
#[derive(Deserialize, Debug)]
pub struct CommandRequest {
    pub channel_name: String,
    pub text: String,
    pub response_url: String,
}

#[derive(Serialize, Debug)]
pub struct CommandResponse {
    // #[serde(rename = "type")]
    pub response_type: String,
    pub text: String,
}

pub async fn execute(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<Value>, hyper::StatusCode> {
    log::trace!("received command: {}", body);

    if !super::verify_signature(headers, body.clone(), &state.secret) {
        return Err(hyper::StatusCode::UNAUTHORIZED);
    }

    let payload = serde_urlencoded::from_str::<CommandRequest>(&body).unwrap();
    let args = payload.text.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    let result = match &args[..space_idx] {
        "list" => handle_list(state.repo.clone(), payload.channel_name).await,
        "create" => handle_create(),
        "edit" => {
            handle_edit(
                state.repo.clone(),
                payload.channel_name,
                &args[space_idx..].trim(),
            )
            .await
        }
        "delete" => {
            handle_delete(
                state.repo.clone(),
                payload.channel_name,
                &args[space_idx..].trim(),
            )
            .await
        }
        "show" => {
            handle_show(
                state.repo.clone(),
                payload.channel_name,
                &args[space_idx..].trim(),
            )
            .await
        }
        "pick" => {
            handle_pick(
                state.repo.clone(),
                payload.response_url.clone(),
                payload.channel_name,
                &args[space_idx..].trim(),
            )
            .await
        }
        "help" => handle_help(&args[space_idx..].trim()),
        _ => {
            let err = to_response_error(UNKNOWN_COMMAND_STR)?;

            super::send_post(&payload.response_url, hyper::Body::from(err))
                .await
                .map_err(|err| {
                    log::error!("unable to send slack error response: {}", err);
                    hyper::StatusCode::INTERNAL_SERVER_ERROR
                })?;

            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    };

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            let err = format!(
                "Error {}: {}.",
                err.as_str(),
                err.canonical_reason().unwrap_or("Unknown")
            );
            let err = to_response_error(&err)?;

            super::send_post(&payload.response_url, hyper::Body::from(err))
                .await
                .map_err(|err| {
                    log::error!("unable to send slack error response: {}", err);
                    hyper::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            return Err(hyper::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match serde_json::from_str(&result) {
        Ok(result) => Ok(Json(result)),
        Err(err) => {
            log::error!("unable to send slack response: {}", err);
            Err(hyper::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_list(
    repo: Arc<dyn Repository>,
    channel: String,
) -> Result<String, hyper::StatusCode> {
    Ok(templates::list_events(repo, channel).await?)
}

fn handle_create() -> Result<String, hyper::StatusCode> {
    Ok(templates::add_event()?)
}

async fn handle_edit(
    repo: Arc<dyn Repository>,
    channel: String,
    args: &str,
) -> Result<String, hyper::StatusCode> {
    if args.len() == 0 {
        return Ok(templates::edit_select_event(repo, channel).await?);
    }

    let id: u32 = match args.parse() {
        Ok(id) => id,
        Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
    };
    Ok(templates::edit_event(repo, channel, id).await?)
}

async fn handle_delete(
    repo: Arc<dyn Repository>,
    channel: String,
    args: &str,
) -> Result<String, hyper::StatusCode> {
    if args.len() == 0 {
        return Ok(templates::delete_select_event(repo, channel).await?);
    }

    let id: u32 = match args.parse() {
        Ok(id) => id,
        Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
    };
    Ok(templates::delete_event(repo, channel, id).await?)
}

async fn handle_show(
    repo: Arc<dyn Repository>,
    channel: String,
    args: &str,
) -> Result<String, hyper::StatusCode> {
    if args.len() == 0 {
        return Ok(templates::show_select_event(repo, channel).await?);
    }

    let id: u32 = match args.parse() {
        Ok(id) => id,
        Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
    };
    Ok(templates::show_event(repo, channel, id).await?)
}

async fn handle_pick(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    args: &str,
) -> Result<String, hyper::StatusCode> {
    if args.len() == 0 {
        return Ok(templates::pick_select_event(repo, channel).await?);
    }

    let id: u32 = match args.parse() {
        Ok(id) => id,
        Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let participant = match pick_participant::execute(
        repo.clone(),
        pick_participant::Request {
            event: id,
            channel: channel.clone(),
        },
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(match err {
                pick_participant::Error::Empty => hyper::StatusCode::NOT_ACCEPTABLE,
                pick_participant::Error::NotFound => hyper::StatusCode::NOT_FOUND,
                pick_participant::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    };

    log::trace!("picked new participant: {:?}", participant);

    let result = templates::pick(repo, channel, id, participant.into(), true).await?;
    super::send_post(&response_url, hyper::Body::from(result))
        .await
        .map_err(|err| {
            log::error!("unable to send slack response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(templates::repick(id)?)
}

fn handle_help(args: &str) -> Result<String, hyper::StatusCode> {
    to_response(match &args.trim()[..] {
        "create" => USAGE_ADD_STR,
        "delete" => USAGE_DELETE_STR,
        "edit" => USAGE_EDIT_STR,
        "list" => USAGE_LIST_STR,
        "pick" => USAGE_PICK_STR,
        "show" => USAGE_SHOW_STR,
        _ => USAGE_STR,
    })
}

fn to_response(value: &str) -> Result<String, hyper::StatusCode> {
    Ok(json!({ "text": value }).to_string())
}

fn to_response_error(value: &str) -> Result<String, hyper::StatusCode> {
    Ok(json!({ "text": value, "response_type": "ephemeral" }).to_string())
}

const USAGE_ADD_STR: &'static str = r#"
`create`     Create a new event
USAGE:
    /picker create
"#;

const USAGE_EDIT_STR: &'static str = r#"
`edit`    Edits an entity
USAGE:
    /picker edit <id>

ARGS:
    <id>    The ID of the event
"#;

const USAGE_DELETE_STR: &'static str = r#"
`del`     Deletes an event
USAGE:
    /picker delete <id>

ARGS:
    <id>    The ID of the event
"#;

const USAGE_LIST_STR: &'static str = r#"
`list`    Lists all the events
USAGE:
    /picker list channels
    /picker list events
"#;

const USAGE_SHOW_STR: &'static str = r#"
`show`    Shows the details of an event
USAGE:
    /picker show <id>

ARGS:
    <id>       The ID of the event
"#;

const USAGE_PICK_STR: &'static str = r#"
`pick`    Picks a random participant for an event
USAGE:
    /picker pick <id>

ARGS:
    <id>       The ID of the event
"#;

const USAGE_STR: &'static str = r#"
USAGE:
`/picker` [SUBCOMMAND] [ARGS]

SUBCOMMANDS:
`create`      Create a new event
`delete`      Deletes an existing event
`edit`        Edits an existing event
`help`        Prints this message or the help of the given subcommand(s)
`list`        Lists all the events
`pick`        Picks randomly a participant of an event
`show`        Shows the details of the event

For more information on a specific command, use `/picker help <command>`
"#;

const UNKNOWN_COMMAND_STR: &'static str = "Sorry but we couldn't find any match command. Please type `/picker help` for all available commands";
