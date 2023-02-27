use std::sync::Arc;

use axum::{extract::State, Json};
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    domain::{
        create_event, delete_event, delete_participants, find_all_channels, find_all_events,
        find_event, pick_participant, repick_participant, update_event, update_participants,
    },
    repository::event::Repository,
};

use super::AppState;

/// Slack command
#[derive(Deserialize, Debug)]
pub struct CommandRequest {
    pub channel_name: String,
    pub text: String,
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
) -> Json<Value> {
    log::trace!("received command: {}", body);

    if !super::verify_signature(headers, body.clone(), &state.secret) {
        return Json(
            serde_json::to_value(CommandResponse {
                response_type: "in_channel".to_string(),
                text: "Failed to authenticate".to_string(),
            })
            .unwrap(),
        );
    }

    let payload = serde_urlencoded::from_str::<CommandRequest>(&body).unwrap();
    let args = payload.text.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    let result = match &args[..space_idx] {
        "test" => return Json(serde_json::from_str(&handle_test()).unwrap()),
        "add" => {
            handle_add(
                state.repo.clone(),
                &payload.channel_name,
                &args[space_idx..],
            )
            .await
        }
        "edit" => {
            handle_edit(
                state.repo.clone(),
                &payload.channel_name,
                &args[space_idx..],
            )
            .await
        }
        "del" => {
            handle_delete(
                state.repo.clone(),
                &payload.channel_name,
                &args[space_idx..],
            )
            .await
        }
        "list" => {
            handle_list(
                state.repo.clone(),
                &payload.channel_name,
                &args[space_idx..],
            )
            .await
        }
        "show" => {
            handle_show(
                state.repo.clone(),
                &payload.channel_name,
                &args[space_idx..],
            )
            .await
        }
        "help" => handle_help(&args[space_idx..]).await,
        "pick" => {
            handle_pick(
                state.repo.clone(),
                &payload.channel_name,
                &args[space_idx..],
            )
            .await
        }
        "repick" => {
            handle_repick(
                state.repo.clone(),
                &payload.channel_name,
                &args[space_idx..],
            )
            .await
        }
        _ => USAGE_STR.to_string(),
    };

    Json(
        serde_json::to_value(&CommandResponse {
            response_type: "in_channel".to_string(),
            text: result,
        })
        .unwrap(),
    )
}

fn handle_test() -> String {
    return std::fs::read_to_string("src/assets/add_event_form.json").expect("Could not read add_event_form.json file");
}

async fn handle_pick(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let id: u32 = match args.trim().parse() {
        Ok(id) => id,
        Err(..) => return "please insert a valid event id".to_string(),
    };
    match pick_participant::execute(
        repo,
        pick_participant::Request {
            event: id,
            channel: channel_name.to_string(),
        },
    )
    .await
    {
        Ok(res) => format!("Picked: <{}>", res.name),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_repick(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let id: u32 = match args.trim().parse() {
        Ok(id) => id,
        Err(..) => return "please insert a valid event id".to_string(),
    };
    match repick_participant::execute(
        repo,
        repick_participant::Request {
            event: id,
            channel: channel_name.to_string(),
        },
    )
    .await
    {
        Ok(res) => format!("Picked: <{}>", res.name),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_add(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let args = args.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    match &args[..space_idx] {
        "event" => handle_add_event(repo, channel_name, &args[space_idx..]).await,
        _ => USAGE_ADD_STR.to_string(),
    }
}

async fn handle_add_event(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let mut event_req: create_event::Request = match serde_json::from_str(args.trim()) {
        Ok(req) => req,
        Err(error) => return error.to_string(),
    };
    event_req.channel = channel_name.to_string();
    match create_event::execute(repo, event_req).await {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_edit(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let args = args.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    match &args[..space_idx] {
        "event" => handle_edit_event(repo, channel_name, &args[space_idx..]).await,
        "participants" => handle_edit_participants(repo, channel_name, &args[space_idx..]).await,
        _ => USAGE_EDIT_STR.to_string(),
    }
}

async fn handle_edit_event(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let mut event_req: update_event::Request = match serde_json::from_str(args.trim()) {
        Ok(req) => req,
        Err(error) => return error.to_string(),
    };
    event_req.channel = channel_name.to_string();
    match update_event::execute(repo, event_req).await {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_edit_participants(
    repo: Arc<dyn Repository>,
    channel_name: &str,
    args: &str,
) -> String {
    let args = args.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    let id: u32 = match args[..space_idx].parse() {
        Ok(id) => id,
        Err(..) => return "please insert a valid event id".to_string(),
    };

    let args = args[space_idx..].trim();
    let participants = match serde_json::from_str::<Vec<String>>(args) {
        Ok(req) => req
            .iter()
            .map(|v| v.trim().to_string())
            .collect::<Vec<String>>(),
        Err(error) => return format!("{:?}", error),
    };
    match update_participants::execute(
        repo,
        update_participants::Request {
            event: id,
            channel: channel_name.to_string(),
            participants,
        },
    )
    .await
    {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_delete(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let args = args.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    match &args[..space_idx] {
        "event" => handle_delete_event(repo, channel_name, &args[space_idx..]).await,
        "participants" => handle_delete_participants(repo, channel_name, &args[space_idx..]).await,
        _ => USAGE_DELETE_STR.to_string(),
    }
}

async fn handle_delete_event(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let id: u32 = match args.trim().parse() {
        Ok(id) => id,
        Err(..) => return "please insert a valid event id".to_string(),
    };
    match delete_event::execute(
        repo,
        delete_event::Request {
            id,
            channel: channel_name.to_string(),
        },
    )
    .await
    {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_delete_participants(
    repo: Arc<dyn Repository>,
    channel_name: &str,
    args: &str,
) -> String {
    let args = args.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    let id: u32 = match args[..space_idx].parse() {
        Ok(id) => id,
        Err(..) => return "please insert a valid event id".to_string(),
    };

    let args = args[space_idx..].trim();
    let participants = match serde_json::from_str::<Vec<String>>(args.trim()) {
        Ok(req) => req
            .iter()
            .map(|v| v.trim().to_string())
            .collect::<Vec<String>>(),
        Err(error) => return format!("{:?}", error),
    };
    match delete_participants::execute(
        repo,
        delete_participants::Request {
            event: id,
            channel: channel_name.to_string(),
            participants,
        },
    )
    .await
    {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_list(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let args = args.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    match &args[..space_idx] {
        "events" => handle_list_events(repo, channel_name).await,
        "channels" => handle_list_channels(repo).await,
        _ => USAGE_LIST_STR.to_string(),
    }
}

async fn handle_list_events(repo: Arc<dyn Repository>, channel_name: &str) -> String {
    match find_all_events::execute(
        repo,
        find_all_events::Request {
            channel: channel_name.to_string(),
        },
    )
    .await
    {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_list_channels(repo: Arc<dyn Repository>) -> String {
    match find_all_channels::execute(repo).await {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_show(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let args = args.trim();
    let space_idx = args.find(' ').unwrap_or(args.len());

    match &args[..space_idx] {
        "event" => handle_show_event(repo, channel_name, &args[space_idx..]).await,
        _ => USAGE_SHOW_STR.to_string(),
    }
}

async fn handle_show_event(repo: Arc<dyn Repository>, channel_name: &str, args: &str) -> String {
    let id: u32 = match args.trim().parse() {
        Ok(id) => id,
        Err(..) => return "please insert a valid event id".to_string(),
    };
    match find_event::execute(
        repo,
        find_event::Request {
            id,
            channel: channel_name.to_string(),
        },
    )
    .await
    {
        Ok(res) => serde_json::to_string(&res).expect("parsing response"),
        Err(error) => format!("{:?}", error),
    }
}

async fn handle_help(args: &str) -> String {
    match &args.trim()[..] {
        "add" => USAGE_ADD_STR.to_string(),
        "del" => USAGE_DELETE_STR.to_string(),
        "edit" => USAGE_EDIT_STR.to_string(),
        "list" => USAGE_LIST_STR.to_string(),
        "pick" => USAGE_PICK_STR.to_string(),
        "repick" => USAGE_REPICK_STR.to_string(),
        "show" => USAGE_SHOW_STR.to_string(),
        _ => USAGE_STR.to_string(),
    }
}

const USAGE_ADD_STR: &'static str = r#"
`add`     Adds an entity
    USAGE:
        /picker add event <event-data>

ARGS:
    <event-data>          Event JSON object with the event creation data

    PROPERTIES:
        <name>          The name of the event
        <date>          The date of the event (in format yyyy-mm-dd)
        <repeat>        Sets if the event should be repeated daily, weekly, bi-weekly, monthly or yearly [possible values: daily, weekly, weekly_two, monthly, yearly]
        <participants>  The participants of the event (multiple values allowed)

    EXAMPLE:
        ```
        {
            "name": "event name",
            "date": "2023-02-10",
            "repeat": "daily",
            "participants": [
                "user1",
                "user2",
                "user3"
            ]
        }
        ```
"#;

const USAGE_EDIT_STR: &'static str = r#"
`edit`    Edits an entity
USAGE:
    /picker edit event <id> <event-data>
    /picker edit participants <id> <participants-data>

ARGS:
    <event-data>            Event JSON object with the event creation data - must also include the id
    <participants-data>     Participants JSON array with the name of the participants to be added in an event
"#;

const USAGE_DELETE_STR: &'static str = r#"
`del`     Deletes an entity
USAGE:
    /picker del <event> <id>
    /picker del <participants> <id> <participants-data>

ARGS:
    <id>                    The ID of the event to delete or change
    <participants>          The participants of the event to remove (multiple values allowed)
"#;

const USAGE_LIST_STR: &'static str = r#"
`list`    Lists entities
USAGE:
    /picker list channels
    /picker list events
"#;

const USAGE_SHOW_STR: &'static str = r#"
`show`    Shows an entity
USAGE:
    /picker show event <id>

ARGS:
    <id>       The ID of the event to show
"#;

const USAGE_PICK_STR: &'static str = r#"
`pick`    Picks a participant for an event
USAGE:
    /picker pick <id>

ARGS:
    <id>       The ID of the event
"#;

const USAGE_REPICK_STR: &'static str = r#"
`repick`  Repicks a participant for an event
USAGE:
    /picker repick <id>

ARGS:
    <id>       The ID of the event
"#;

const USAGE_STR: &'static str = r#"
USAGE:
    `/picker` [SUBCOMMAND] [ARGS]

SUBCOMMANDS:
    `add`         Adds an entity
    `del`         Deletes an entity
    `edit`        Edits an entity
    `help`        Prints this message or the help of the given subcommand(s)
    `list`        Lists entities
    `pick`        Picks an event
    `repick`      Repicks an event
    `show`        Shows an entity

For more information on a specific command, use `/picker help <command>`
"#;
