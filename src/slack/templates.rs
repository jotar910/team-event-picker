use std::sync::Arc;

use hyper::StatusCode;
use serde_json::{json, Value};

use crate::{
    domain::{
        events::{find_all_events, find_event},
        timezone::Timezone,
    },
    repository::event::Repository,
    slack::helpers,
};

pub fn add_event() -> Result<String, Error> {
    let template = read_file(ADD_EVENT_HBS)?;
    let result = super::render_template(&template, json!({ "timezones": Timezone::options() }))
        .map_err(|err| {
            log::error!("could not render template {}: {}", ADD_EVENT_HBS, err);
            Error::ReadFile
        })?;

    Ok(result)
}

pub async fn add_event_success(
    repo: Arc<dyn Repository>,
    channel: String,
    id: u32,
) -> Result<String, Error> {
    event_action_success(repo, channel, id, ADD_EVENT_SUCCESS_HBS).await
}

pub async fn edit_event(
    repo: Arc<dyn Repository>,
    channel: String,
    id: u32,
) -> Result<String, Error> {
    let event = find_event::execute(repo, find_event::Request { id, channel }).await?;

    let template = read_file(EDIT_EVENT_HBS)?;
    let result = super::render_template(
        &template,
        json!({
            "id": event.id,
            "name": event.name,
            "date": event.timestamp,
            "repeat": event.repeat.clone().try_into().unwrap_or(String::from("")),
            "repeat_label": event.repeat.label(),
            "participants": event.participants.into_iter().map(|p| p.user).collect::<Vec<String>>(),
            "timezone": event.timezone.clone().option(),
            "timezones": Timezone::options()
        }),
    )
    .map_err(|err| {
        log::error!("could not render template {}: {}", EDIT_EVENT_HBS, err);
        Error::ReadFile
    })?;

    Ok(result)
}

pub async fn edit_event_success(
    repo: Arc<dyn Repository>,
    channel: String,
    id: u32,
) -> Result<String, Error> {
    event_action_success(repo, channel, id, EDIT_EVENT_SUCCESS_HBS).await
}

pub async fn edit_select_event(
    repo: Arc<dyn Repository>,
    channel: String,
) -> Result<String, Error> {
    select_event(repo, channel, EDIT_SELECT_EVENT_HBS).await
}

pub async fn delete_event(
    repo: Arc<dyn Repository>,
    channel: String,
    id: u32,
) -> Result<String, Error> {
    let event = find_event::execute(repo, find_event::Request { id, channel }).await?;

    let template = read_file(DELETE_EVENT_HBS)?;
    let result = super::render_template(
        &template,
        json!({
            "name": event.name,
            "id": event.id
        }),
    )
    .map_err(|err| {
        log::error!("could not render template {}: {}", DELETE_EVENT_HBS, err);
        Error::ReadFile
    })?;

    Ok(result)
}

pub async fn delete_event_success() -> Result<String, Error> {
    Ok(json!({
        "text": "Event deleted with success! 👍"
    })
    .to_string())
}

pub async fn delete_select_event(
    repo: Arc<dyn Repository>,
    channel: String,
) -> Result<String, Error> {
    select_event(repo, channel, DELETE_SELECT_EVENT_HBS).await
}

pub async fn show_event(
    repo: Arc<dyn Repository>,
    channel: String,
    id: u32,
) -> Result<String, Error> {
    let event = find_event::execute(repo, find_event::Request { id, channel }).await?;

    let template = read_file(SHOW_EVENT_HBS)?;
    let result = super::render_template(
        &template,
        json!({
            "id": event.id,
            "name": event.name,
            "date": helpers::fmt_timestamp(event.timestamp, event.timezone),
            "repeat": event.repeat.to_string(),
            "participants": event.participants.into_iter().map(|p| p.user).collect::<Vec<String>>()
        }),
    )
    .map_err(|err| {
        log::error!("could not render template {}: {}", SHOW_EVENT_HBS, err);
        Error::ReadFile
    })?;

    Ok(result)
}

pub async fn show_select_event(
    repo: Arc<dyn Repository>,
    channel: String,
) -> Result<String, Error> {
    select_event(repo, channel, SHOW_SELECT_EVENT_HBS).await
}

pub async fn pick_select_event(
    repo: Arc<dyn Repository>,
    channel: String,
) -> Result<String, Error> {
    select_event(repo, channel, PICK_SELECT_EVENT_HBS).await
}

async fn select_event(
    repo: Arc<dyn Repository>,
    channel: String,
    filename: &str,
) -> Result<String, Error> {
    let events = find_all_events::execute(repo.clone(), find_all_events::Request { channel })
        .await?
        .data;

    let template = read_file(filename)?;
    let result = super::render_template(
        &template,
        json!({
            "events": events
                .into_iter()
                .map(|event|
                    json!({
                        "text": format!("[{}]: {}", event.id, event.name),
                        "id": event.id
                    })
                )
                .collect::<Vec<Value>>(),
        }),
    )
    .map_err(|err| {
        log::error!("could not render template {}: {}", filename, err);
        Error::RenderTemplate
    })?;

    Ok(result)
}

async fn event_action_success(
    repo: Arc<dyn Repository>,
    channel: String,
    id: u32,
    filename: &str,
) -> Result<String, Error> {
    let event = find_event::execute(repo, find_event::Request { channel, id }).await?;

    let template = read_file(filename)?;
    let result = super::render_template(
        &template,
        json!({
            "id": event.id,
            "name": event.name,
            "date": helpers::fmt_timestamp(event.timestamp, event.timezone),
            "repeat": event.repeat.to_string(),
            "participants": event.participants.into_iter().map(|p| p.user).collect::<Vec<String>>()
        }),
    )
    .map_err(|err| {
        log::error!("could not render template {}: {}", filename, err);
        Error::RenderTemplate
    })?;

    Ok(result)
}

pub enum Error {
    Query,
    QueryNotFound,
    ReadFile,
    RenderTemplate,
}

impl From<Error> for StatusCode {
    fn from(value: Error) -> Self {
        match value {
            Error::QueryNotFound => Self::NOT_FOUND,
            Error::ReadFile | Error::Query | Error::RenderTemplate => Self::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<find_event::Error> for Error {
    fn from(value: find_event::Error) -> Self {
        match value {
            find_event::Error::NotFound => Self::QueryNotFound,
            find_event::Error::Unknown => Self::Query,
        }
    }
}

impl From<find_all_events::Error> for Error {
    fn from(value: find_all_events::Error) -> Self {
        match value {
            find_all_events::Error::Unknown => Self::Query,
        }
    }
}

const HBS_BASE_PATHS: &str = "src/assets";
const ADD_EVENT_HBS: &str = "add_event.json.hbs";
const ADD_EVENT_SUCCESS_HBS: &str = "add_event_success.json.hbs";
const EDIT_EVENT_HBS: &str = "edit_event.json.hbs";
const EDIT_EVENT_SUCCESS_HBS: &str = "edit_event_success.json.hbs";
const EDIT_SELECT_EVENT_HBS: &str = "edit_select_event.json.hbs";
const DELETE_EVENT_HBS: &str = "delete_event.json.hbs";
const DELETE_SELECT_EVENT_HBS: &str = "delete_select_event.json.hbs";
const SHOW_EVENT_HBS: &str = "show_event.json.hbs";
const SHOW_SELECT_EVENT_HBS: &str = "show_select_event.json.hbs";
const PICK_SELECT_EVENT_HBS: &str = "pick_select_event.json.hbs";

fn hbs_path(filename: &str) -> String {
    format!("{}/{}", HBS_BASE_PATHS, filename)
}

fn read_file(filename: &str) -> Result<String, Error> {
    std::fs::read_to_string(hbs_path(filename)).map_err(|err| {
        log::error!("could not read file {}: {}", filename, err);
        Error::ReadFile
    })
}
