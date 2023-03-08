use std::{collections::HashMap, sync::Arc};

use axum::extract::{Form, State};
use chrono::TimeZone;
use chrono::Utc;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::domain::delete_event;
use crate::domain::pick_participant;
use crate::domain::repick_participant;
use crate::{
    domain::{create_event, update_event},
    repository::event::Repository,
};

use super::templates;
use super::AppState;

#[derive(Serialize, Deserialize)]
pub struct CommandActionBody {
    payload: String,
}

/// Slack action
#[derive(Deserialize, Debug, Clone)]
pub struct CommandAction {
    #[serde(rename = "type")]
    request_type: String,
    response_url: String,
    channel: Channel,
    state: FormState,
    actions: Vec<Action>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Channel {
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Action {
    action_id: Option<String>,
    block_id: Option<String>,
    value: Option<String>,
    selected_option: Option<SelectedOption>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FormState {
    values: FormStateValues,
}

type FormStateValues = HashMap<String, FormStateValue>;

#[derive(Deserialize, Debug, Clone)]
pub struct FormStateValue {
    name_input: Option<InputText>,
    date_input: Option<DateTimePicker>,
    repeat_input: Option<RadioButton>,
    participants_input: Option<MultiUsersSelect>,
    select_event: Option<StaticSelect>,
}

impl FormStateValue {
    fn new() -> FormStateValue {
        Self {
            name_input: None,
            date_input: None,
            repeat_input: None,
            participants_input: None,
            select_event: None,
        }
    }

    fn merge(self, v: FormStateValue) -> FormStateValue {
        Self {
            name_input: merge_option(self.name_input, v.name_input),
            date_input: merge_option(self.date_input, v.date_input),
            repeat_input: merge_option(self.repeat_input, v.repeat_input),
            participants_input: merge_option(self.participants_input, v.participants_input),
            select_event: merge_option(self.select_event, v.select_event),
        }
    }
}

impl From<FormState> for FormStateValue {
    fn from(form: FormState) -> Self {
        form.values
            .into_iter()
            .fold(FormStateValue::new(), |acc, (_, v)| acc.merge(v))
    }
}

fn merge_option<T>(acc: Option<T>, cur: Option<T>) -> Option<T> {
    match acc {
        Some(..) => acc,
        None => cur,
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct InputText {
    value: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DateTimePicker {
    selected_date_time: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RadioButton {
    selected_option: Option<SelectedOption>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SelectedOption {
    value: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MultiUsersSelect {
    selected_users: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StaticSelect {
    selected_option: Option<SelectedOption>,
}

#[derive(Serialize, Debug)]
pub struct CommandActionResponse {
    // #[serde(rename = "type")]
    response_type: String,
    text: String,
}

#[derive(Clone)]
struct AddEventData {
    channel: String,
    form: FormStateValue,
}

impl AddEventData {
    fn new(value: CommandAction) -> Self {
        Self {
            channel: value.channel.name,
            form: value.state.into(),
        }
    }
}

impl TryFrom<AddEventData> for create_event::Request {
    type Error = String;

    fn try_from(data: AddEventData) -> Result<Self, Self::Error> {
        Ok(create_event::Request {
            channel: data.channel,
            name: data
                .form
                .name_input
                .ok_or("no name input")?
                .value
                .ok_or("no name value")?,
            date: Utc
                .timestamp_opt(
                    data.form
                        .date_input
                        .ok_or("no date input")?
                        .selected_date_time
                        .ok_or("no date value")?,
                    0,
                )
                .unwrap()
                .to_string(),
            repeat: data
                .form
                .repeat_input
                .clone()
                .ok_or("no repeat input")?
                .selected_option
                .ok_or("no repeat option")?
                .value
                .ok_or("no repeat value")?,
            participants: data
                .form
                .participants_input
                .ok_or("no participants input")?
                .selected_users,
        })
    }
}

#[derive(Clone)]
struct UpdateEventData {
    event_id: u32,
    add_event: AddEventData,
}

impl UpdateEventData {
    fn new(event_id: u32, value: CommandAction) -> Self {
        Self {
            event_id,
            add_event: AddEventData::new(value),
        }
    }
}

impl TryFrom<UpdateEventData> for update_event::Request {
    type Error = String;

    fn try_from(data: UpdateEventData) -> Result<Self, Self::Error> {
        let edit_event: create_event::Request = data.add_event.try_into()?;
        Ok(update_event::Request {
            id: data.event_id,
            channel: edit_event.channel,
            name: edit_event.name,
            date: edit_event.date,
            repeat: edit_event.repeat,
            participants: edit_event.participants,
        })
    }
}

struct SelectEventData {
    id: u32,
}

impl SelectEventData {
    fn try_new(value: CommandAction) -> Result<Self, String> {
        let form: FormStateValue = value.state.into();
        Ok(Self {
            id: form
                .select_event
                .ok_or("no selected event")?
                .selected_option
                .ok_or("no selected option")?
                .value
                .ok_or("no selected option value")?
                .parse()
                .map_err(|_| "invalid selected value")?,
        })
    }
}

pub async fn execute(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Form(payload): Form<CommandActionBody>,
) -> Result<(), hyper::StatusCode> {
    let body = serde_urlencoded::to_string(&payload).unwrap();
    log::trace!("received action: {}", body);

    if !super::verify_signature(headers, body.clone(), &state.secret) {
        return Err(hyper::StatusCode::UNAUTHORIZED);
    }

    let payload: CommandAction = serde_json::from_str(&payload.payload).unwrap();

    if payload.request_type != "block_actions" {
        log::trace!("unknown action type: {}", payload.request_type);
        return Ok(());
    }

    for action in payload.actions.iter() {
        if let None = action.block_id {
            continue;
        }
        let result = match action.block_id.as_deref().unwrap() {
            "add_event_actions" => handle_add_event(state.repo.clone(), action, &payload).await,
            "edit_event_actions" => handle_edit_event(state.repo.clone(), action, &payload).await,
            "select_event_edit_actions" => {
                handle_edit_select_event(state.repo.clone(), action, &payload).await
            }
            "delete_event_actions" => {
                handle_delete_event(state.repo.clone(), action, &payload).await
            }
            "select_event_delete_actions" => {
                handle_delete_select_event(state.repo.clone(), action, &payload).await
            }
            "select_event_pick_actions" => {
                handle_pick_select_event(state.repo.clone(), action, &payload).await
            }
            "select_event_show_actions" => {
                handle_show_select_event(state.repo.clone(), action, &payload).await
            }
            "list_events_actions" => handle_list_event(action, &payload).await,
            "show_event_actions" | "add_event_success_action" | "edit_event_success_action" => {
                handle_show_event(state.repo.clone(), action, &payload).await
            }
            id => {
                let id = match id.parse::<u32>() {
                    Ok(id) => id,
                    Err(..) => continue,
                };
                if let None = action.action_id {
                    continue;
                }
                match action.action_id.as_deref().unwrap() {
                    "list_event_actions" => {
                        handle_list_item_event(state.repo.clone(), action, &payload, id).await
                    }
                    "repick_event" => {
                        handle_repick_event(
                            state.repo.clone(),
                            payload.response_url,
                            payload.channel.name,
                            id,
                        )
                        .await
                    }
                    _ => continue,
                }
            }
        };
        if let Err(err) = result {
            log::trace!("failed to execute action: {}", err);
            return Err(err);
        }
        return Ok(());
    }

    log::trace!("unknown action: {:?}", payload);

    Ok(())
}

async fn handle_add_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let request: create_event::Request = match AddEventData::new(command_action.clone()).try_into()
    {
        Ok(data) => data,
        Err(err) => {
            log::trace!("error parsing data to create event request: {}", err);
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    };
    let response = match create_event::execute(repo.clone(), request).await {
        Ok(res) => res,
        Err(create_event::Error::BadRequest) => return Err(hyper::StatusCode::BAD_REQUEST),
        Err(create_event::Error::Conflict) => return Err(hyper::StatusCode::CONFLICT),
        _ => return Err(hyper::StatusCode::INTERNAL_SERVER_ERROR),
    };

    let body = templates::add_event_success(repo, command_action.channel.name.clone(), response.id)
        .await?;
    super::send_post(&command_action.response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_edit_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let event_id: u32 = match action.action_id.clone() {
        Some(id) => match id.parse() {
            Ok(id) => id,
            Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let request: update_event::Request =
        match UpdateEventData::new(event_id, command_action.clone()).try_into() {
            Ok(data) => data,
            Err(err) => {
                log::trace!("error parsing data to update event request: {}", err);
                return Err(hyper::StatusCode::BAD_REQUEST);
            }
        };
    let response = match update_event::execute(repo.clone(), request).await {
        Ok(res) => res,
        Err(update_event::Error::BadRequest) => return Err(hyper::StatusCode::BAD_REQUEST),
        Err(update_event::Error::Conflict) => return Err(hyper::StatusCode::CONFLICT),
        Err(update_event::Error::NotFound) => return Err(hyper::StatusCode::NOT_FOUND),
        _ => return Err(hyper::StatusCode::INTERNAL_SERVER_ERROR),
    };

    let body =
        templates::edit_event_success(repo, command_action.channel.name.clone(), response.id)
            .await?;
    super::send_post(&command_action.response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_edit_select_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let event_id: u32 = match SelectEventData::try_new(command_action.clone()) {
        Ok(select) => select.id,
        Err(err) => {
            log::trace!("error to find event id from action data: {}", err);
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    };

    handle_edit_selected_event(
        repo,
        command_action.response_url.clone(),
        command_action.channel.name.clone(),
        event_id,
    )
    .await
}

async fn handle_delete_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let event_id: u32 = match action.action_id.clone() {
        Some(id) => match id.parse() {
            Ok(id) => id,
            Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let request = delete_event::Request {
        id: event_id,
        channel: command_action.channel.name.clone(),
    };
    match delete_event::execute(repo.clone(), request).await {
        Ok(..) => (),
        Err(delete_event::Error::NotFound) => return Err(hyper::StatusCode::NOT_FOUND),
        _ => return Err(hyper::StatusCode::INTERNAL_SERVER_ERROR),
    };

    let body = templates::delete_event_success().await?;
    super::send_post(&command_action.response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_delete_select_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let event_id: u32 = match SelectEventData::try_new(command_action.clone()) {
        Ok(select) => select.id,
        Err(err) => {
            log::trace!("error to find event id from action data: {}", err);
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    };

    handle_delete_selected_event(
        repo,
        command_action.response_url.clone(),
        command_action.channel.name.clone(),
        event_id,
    )
    .await
}

async fn handle_pick_select_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let event_id: u32 = match SelectEventData::try_new(command_action.clone()) {
        Ok(select) => select.id,
        Err(err) => {
            log::trace!("error to find event id from action data: {}", err);
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    };

    handle_pick_event(
        repo,
        command_action.response_url.clone(),
        command_action.channel.name.clone(),
        event_id,
    )
    .await
}

async fn handle_list_event(
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    match action.value.clone() {
        Some(value) if value == "cancel" => handle_close(&command_action.response_url).await,
        Some(value) if value == "add_event" => {
            handle_create_event(&command_action.response_url).await
        }
        _ => return Err(hyper::StatusCode::BAD_REQUEST),
    }
}

async fn handle_list_item_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    let response_url = command_action.response_url.clone();
    let channel = command_action.channel.name.clone();
    let selected_option = match action.selected_option.clone() {
        Some(option) => match option.value {
            Some(option) => option,
            None => return Err(hyper::StatusCode::BAD_REQUEST),
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };
    match selected_option.as_str() {
        "pick" => handle_pick_event(repo, response_url, channel, event_id).await,
        "show" => handle_show_details_event(repo, response_url, channel, event_id).await,
        "edit" => handle_edit_selected_event(repo, response_url, channel, event_id).await,
        "delete" => handle_delete_selected_event(repo, response_url, channel, event_id).await,
        _ => return Err(hyper::StatusCode::BAD_REQUEST),
    }
}

async fn handle_show_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    let action_type = match action.action_id.clone() {
        Some(action_id) if action_id == "cancel" => {
            return handle_close(&command_action.response_url).await
        }
        Some(action_id) => action_id,
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let event_id: u32 = match action.value.clone() {
        Some(value) => match value.parse() {
            Ok(id) => id,
            Err(err) => {
                log::trace!("error retrieving event id from action value: {}", err);
                return Err(hyper::StatusCode::BAD_REQUEST);
            }
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let response_url = command_action.response_url.clone();
    let channel = command_action.channel.name.clone();
    match action_type.as_str() {
        "pick" => handle_pick_event(repo, response_url, channel, event_id).await,
        "edit_event" => handle_edit_selected_event(repo, response_url, channel, event_id).await,
        "delete_event" => handle_delete_selected_event(repo, response_url, channel, event_id).await,
        _ => return Err(hyper::StatusCode::BAD_REQUEST),
    }
}

async fn handle_show_select_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let event_id: u32 = match SelectEventData::try_new(command_action.clone()) {
        Ok(select) => select.id,
        Err(err) => {
            log::trace!("error to find event id from action data: {}", err);
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    };

    handle_show_details_event(
        repo,
        command_action.response_url.clone(),
        command_action.channel.name.clone(),
        event_id,
    )
    .await
}

async fn handle_pick_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    let request = pick_participant::Request {
        event: event_id,
        channel: channel.clone(),
    };
    let response = match pick_participant::execute(repo.clone(), request).await {
        Ok(response) => response,
        Err(err) => {
            return Err(match err {
                pick_participant::Error::Empty => hyper::StatusCode::NOT_ACCEPTABLE,
                pick_participant::Error::NotFound => hyper::StatusCode::NOT_FOUND,
                pick_participant::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    };

    let body = templates::pick(repo, channel, event_id, response.into(), false).await?;
    super::send_post(&response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_repick_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    let request = repick_participant::Request {
        event: event_id,
        channel: channel.clone(),
    };
    let response = match repick_participant::execute(repo.clone(), request).await {
        Ok(response) => response,
        Err(err) => {
            return Err(match err {
                repick_participant::Error::Empty => hyper::StatusCode::NOT_ACCEPTABLE,
                repick_participant::Error::NotFound => hyper::StatusCode::NOT_FOUND,
                repick_participant::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    };

    let body = templates::repick(event_id)?;
    super::send_post(&response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let body = templates::pick(repo, channel, event_id, response.into(), false).await?;
    super::send_post(&response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_create_event(response_url: &str) -> Result<(), hyper::StatusCode> {
    let body = templates::add_event()?;
    super::send_post(&response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_edit_selected_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    let body = templates::edit_event(repo, channel, event_id).await?;
    super::send_post(&response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_delete_selected_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    let body = templates::delete_event(repo, channel, event_id).await?;
    super::send_post(&response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_show_details_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    let body = templates::show_event(repo, channel, event_id).await?;
    super::send_post(&response_url, hyper::Body::from(body))
        .await
        .map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(())
}

async fn handle_close(response_url: &str) -> Result<(), hyper::StatusCode> {
    super::send_post(
        response_url,
        hyper::Body::from(r#"{"delete_original": true}"#),
    )
    .await
    .map_err(|err| {
        log::error!("unable to send slack error response: {}", err);
        hyper::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}
