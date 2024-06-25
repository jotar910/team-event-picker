use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use hyper::HeaderMap;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::{
        commands::{self, pick_participant},
        events::repick_participant,
    },
    repository::event::Repository,
};

use super::{templates, AppState};

/// Slack command
#[derive(Deserialize, Debug)]
pub struct CommandRequest {
    pub channel_id: String,
    pub text: String,
    pub response_url: String,
    pub user_id: String,
}

pub async fn execute(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Response, hyper::StatusCode> {
    log::trace!("received command: \n{:?} \n{}", headers, body);

    let payload = serde_urlencoded::from_str::<CommandRequest>(&body).unwrap();
    let args = payload.text.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    let reached_limit = super::find_reached_limit(&headers)?;

    let result = match &args[..space_idx] {
        "list" => handle_list(state.event_repo.clone(), payload.channel_id, reached_limit).await,
        "create" => handle_create(),
        "edit" => {
            handle_edit(
                state.event_repo.clone(),
                payload.channel_id,
                &args[space_idx..].trim(),
            )
            .await
        }
        "delete" => {
            handle_delete(
                state.event_repo.clone(),
                payload.channel_id,
                &args[space_idx..].trim(),
            )
            .await
        }
        "show" => {
            handle_show(
                state.event_repo.clone(),
                payload.channel_id,
                &args[space_idx..].trim(),
            )
            .await
        }
        "pick" => {
            handle_pick(
                state.event_repo.clone(),
                payload.response_url.clone(),
                payload.channel_id,
                payload.user_id,
                &args[space_idx..].trim(),
            )
            .await
        }
        "repick" => {
            handle_repick(
                state.event_repo.clone(),
                payload.response_url.clone(),
                payload.channel_id,
                &args[space_idx..].trim(),
            )
            .await
        }
        "help" => handle_help(&args[space_idx..].trim()),
        _ => {
            let err = super::to_response_error(UNKNOWN_COMMAND_STR)?;

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
            let err = super::to_response_error(&err)?;

            super::send_post(&payload.response_url, hyper::Body::from(err))
                .await
                .map_err(|err| {
                    log::error!("unable to send slack error response: {}", err);
                    hyper::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            return Err(hyper::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if result.is_empty() {
        return Ok((()).into_response());
    }

    match serde_json::from_str::<Value>(&result) {
        Ok(result) => {
            log::debug!("command response: {:?}", result);
            Ok(Json(result).into_response())
        }
        Err(err) => {
            log::error!("unable to send slack response: {}", err);
            Err(hyper::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_list(
    repo: Arc<dyn Repository>,
    channel: String,
    reached_limit: bool,
) -> Result<String, hyper::StatusCode> {
    Ok(commands::list_events::execute(repo, channel, reached_limit)
        .await?
        .to_string())
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
    user: String,
    args: &str,
) -> Result<String, hyper::StatusCode> {
    if args.len() == 0 {
        return Ok(templates::pick_select_event(repo, channel).await?);
    }

    let id: u32 = match args.parse() {
        Ok(id) => id,
        Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let response = pick_participant::execute(repo.clone(), id, channel, user, response_url)
        .await?
        .map_or(String::from(""), |r| r.to_string());

    return Ok(response);
}

async fn handle_repick(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    args: &str,
) -> Result<String, hyper::StatusCode> {
    let id: u32 = match args.parse() {
        Ok(id) => id,
        Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let participant = match repick_participant::execute(
        repo.clone(),
        repick_participant::Request {
            event: id,
            channel: channel.clone(),
        },
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(match err {
                repick_participant::Error::Empty => hyper::StatusCode::NOT_ACCEPTABLE,
                repick_participant::Error::NotFound => hyper::StatusCode::NOT_FOUND,
                repick_participant::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
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
    super::to_response(match &args.trim()[..] {
        "create" => USAGE_ADD_STR,
        "delete" => USAGE_DELETE_STR,
        "edit" => USAGE_EDIT_STR,
        "list" => USAGE_LIST_STR,
        "pick" => USAGE_PICK_STR,
        "show" => USAGE_SHOW_STR,
        _ => USAGE_STR,
    })
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
