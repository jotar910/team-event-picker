use std::{collections::HashMap, sync::Arc};

use axum::extract::{Form, State};
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::from_str;

use super::state::AppConfigs;
use super::{templates, AppState};
use crate::domain::commands::cancel_pick;
use crate::domain::entities::RepeatPeriod;
use crate::domain::timezone::Timezone;
use crate::scheduler::{entities::EventSchedule, Scheduler};
use crate::{
    domain::commands::{pick_participant, repick_participant},
    domain::events::{create_event, delete_event, find_event, update_event},
    repository::event::Repository,
};

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
    user: User,
    state: FormState,
    actions: Vec<Action>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Channel {
    id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    id: String,
    team_id: String,
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
    timezone_input: Option<StaticSelect>,
    select_event: Option<StaticSelect>,
}

impl FormStateValue {
    fn new() -> FormStateValue {
        Self {
            name_input: None,
            date_input: None,
            repeat_input: None,
            participants_input: None,
            timezone_input: None,
            select_event: None,
        }
    }

    fn merge(self, v: FormStateValue) -> FormStateValue {
        Self {
            name_input: merge_option(self.name_input, v.name_input),
            date_input: merge_option(self.date_input, v.date_input),
            repeat_input: merge_option(self.repeat_input, v.repeat_input),
            participants_input: merge_option(self.participants_input, v.participants_input),
            timezone_input: merge_option(self.timezone_input, v.timezone_input),
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
    team_id: String,
    form: FormStateValue,
    max_events: u32,
}

impl AddEventData {
    fn new(value: CommandAction, max_events: u32) -> Self {
        Self {
            channel: value.channel.id,
            team_id: value.user.team_id,
            form: value.state.into(),
            max_events,
        }
    }
}

impl TryFrom<AddEventData> for create_event::Request {
    type Error = String;

    fn try_from(data: AddEventData) -> Result<Self, Self::Error> {
        let participants = data
            .form
            .participants_input
            .ok_or("no participants input")?
            .selected_users;
        if participants.len() == 0 {
            return Err(String::from("participants is empty"));
        }
        Ok(create_event::Request {
            max_events: data.max_events,
            channel: data.channel,
            team_id: data.team_id,
            name: data
                .form
                .name_input
                .ok_or("no name input")?
                .value
                .ok_or("no name value")?,
            timestamp: data
                .form
                .date_input
                .ok_or("no date input")?
                .selected_date_time
                .ok_or("no date value")?,
            timezone: data
                .form
                .timezone_input
                .and_then(|d| d.selected_option)
                .and_then(|d| d.value)
                .unwrap_or(Timezone::UTC.into()),
            repeat: match data.form.repeat_input {
                Some(input) => input
                    .clone()
                    .selected_option
                    .ok_or("no repeat option")?
                    .value
                    .ok_or("no repeat value")?,
                None => String::try_from(RepeatPeriod::None)?,
            },
            participants,
        })
    }
}

#[derive(Clone)]
struct UpdateEventDetails {
    id: u32,
    name: String,
    timestamp: i64,
    timezone: Timezone,
    repeat: RepeatPeriod,
    participants: Vec<String>,
}

impl From<find_event::Response> for UpdateEventDetails {
    fn from(value: find_event::Response) -> Self {
        Self {
            id: value.id,
            name: value.name,
            timestamp: value.timestamp,
            timezone: value.timezone,
            repeat: value.repeat,
            participants: value.participants.into_iter().map(|p| p.user).collect(),
        }
    }
}

#[derive(Clone)]
struct UpdateEventData {
    event: UpdateEventDetails,
    channel: String,
    form: FormStateValue,
}

impl UpdateEventData {
    fn new(event: UpdateEventDetails, value: CommandAction) -> Self {
        Self {
            event,
            channel: value.channel.id,
            form: value.state.into(),
        }
    }
}

impl TryFrom<UpdateEventData> for update_event::Request {
    type Error = String;

    fn try_from(data: UpdateEventData) -> Result<Self, Self::Error> {
        let participants = data
            .form
            .participants_input
            .map_or(data.event.participants, |d| d.selected_users);
        if participants.len() == 0 {
            return Err(String::from("participants is empty"));
        }

        Ok(update_event::Request {
            id: data.event.id,
            channel: data.channel,
            name: data
                .form
                .name_input
                .and_then(|d| d.value)
                .unwrap_or(data.event.name),
            timestamp: data
                .form
                .date_input
                .and_then(|d| d.selected_date_time)
                .unwrap_or(data.event.timestamp),
            timezone: data
                .form
                .timezone_input
                .and_then(|d| d.selected_option)
                .and_then(|d| d.value)
                .unwrap_or(data.event.timezone.into()),
            repeat: data
                .form
                .repeat_input
                .and_then(|d| d.selected_option)
                .and_then(|d| d.value)
                .unwrap_or(String::try_from(data.event.repeat).unwrap_or(String::from("none"))),
            participants,
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
    log::trace!(
        "received action: \n{:?} \n{:?}",
        headers,
        from_str(&body).unwrap_or(body)
    );

    // let token = super::find_token(&headers)?;

    let payload: CommandAction = from_str(&payload.payload).unwrap();

    if payload.request_type != "block_actions" {
        log::trace!("unknown action type: {}", payload.request_type);
        return Ok(());
    }

    for action in payload.actions.iter() {
        if let Some(action_id) = action.action_id.as_deref() {
            if action_id.starts_with("pick_participant_actions:") {
                return handle_pick_participant_event(state.event_repo.clone(), action, &payload)
                    .await;
            }
            if action_id.starts_with("cancel_pick_actions:") {
                return handle_cancel_pick_event(state.event_repo.clone(), action, &payload).await;
            }
        }
        if let None = action.block_id {
            log::trace!("block id not provided on action");
            continue;
        }
        let result = match action.block_id.as_deref().unwrap() {
            "add_event_actions" => {
                handle_add_event(
                    state.event_repo.clone(),
                    state.scheduler.clone(),
                    state.configs.clone(),
                    // token,
                    action,
                    &payload,
                )
                .await
            }
            "edit_event_actions" => {
                handle_edit_event(
                    state.event_repo.clone(),
                    state.scheduler.clone(),
                    action,
                    &payload,
                )
                .await
            }
            "select_event_edit_actions" => {
                handle_edit_select_event(state.event_repo.clone(), action, &payload).await
            }
            "delete_event_actions" => {
                handle_delete_event(
                    state.event_repo.clone(),
                    state.scheduler.clone(),
                    action,
                    &payload,
                )
                .await
            }
            "select_event_delete_actions" => {
                handle_delete_select_event(state.event_repo.clone(), action, &payload).await
            }
            "select_event_pick_actions" => {
                handle_pick_select_event(state.event_repo.clone(), action, &payload).await
            }
            "select_event_show_actions" => {
                handle_show_select_event(state.event_repo.clone(), action, &payload).await
            }
            "list_events_actions" => handle_list_event(action, &payload).await,
            "show_event_actions" | "add_event_success_action" | "edit_event_success_action" => {
                handle_show_event(state.event_repo.clone(), action, &payload).await
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
                        handle_list_item_event(state.event_repo.clone(), action, &payload, id).await
                    }
                    "repick_event" => {
                        handle_repick_event(
                            state.event_repo.clone(),
                            payload.response_url,
                            payload.channel.id,
                            payload.user.id,
                            id,
                        )
                        .await
                    }
                    _ => continue,
                }
            }
        };
        if let Err(err) = result {
            log::info!("failed to execute action: {}", err);
            return Err(err);
        }
        return Ok(());
    }

    log::trace!("unknown action: {:?}", payload);

    Ok(())
}

async fn handle_add_event(
    repo: Arc<dyn Repository>,
    scheduler: Arc<Scheduler>,
    configs: Arc<AppConfigs>,
    // token: String,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let request: create_event::Request =
        match AddEventData::new(command_action.clone(), configs.max_events).try_into() {
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

    // TODO: Check if needed this extra complexity.
    // let added_to_channel = match response.created_channel {
    //     Some(channel) => {
    //         match sender::join_channel(&token, &channel).await {
    //             Ok(res) => {
    //                 /* TODO: find why this gives error, and putting outside don't.
    //                 if let Err(err) = task::spawn(async move {
    //                     scheduler.insert(EventSchedule {
    //                         id: response.id,
    //                         date: response.date,
    //                         repeat: response.repeat,
    //                     }).await;
    //                 }).await {
    //                     log::error!("unable to insert event into scheduler: {}", err)
    //                 } */
    //                 Some(res)
    //             }
    //             Err(err) => {
    //                 log::error!("unable to send slack error response: {}", err);
    //                 None
    //             }
    //         }
    //     }
    //     None => Some(()),
    // };
    //
    // if let Some(..) = added_to_channel {
    scheduler
        .insert(EventSchedule {
            id: response.id,
            timestamp: response.timestamp,
            timezone: response.timezone,
            repeat: response.repeat,
        })
        .await;
    // }

    let body =
        templates::add_event_success(repo, command_action.channel.id.clone(), response.id).await?;
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
    scheduler: Arc<Scheduler>,
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
    let channel_id = command_action.channel.id.clone();

    let request = find_event::Request {
        id: event_id,
        channel: channel_id,
    };
    let event: UpdateEventDetails = match find_event::execute(repo.clone(), request).await {
        Ok(event) => event.into(),
        Err(err) => {
            return Err(match err {
                find_event::Error::NotFound => hyper::StatusCode::NOT_FOUND,
                find_event::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    };

    let request: update_event::Request =
        match UpdateEventData::new(event, command_action.clone()).try_into() {
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

    scheduler
        .insert(EventSchedule {
            id: response.id,
            timestamp: response.timestamp,
            timezone: response.timezone,
            repeat: response.repeat,
        })
        .await;

    let body =
        templates::edit_event_success(repo, command_action.channel.id.clone(), response.id).await?;
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
        command_action.channel.id.clone(),
        event_id,
    )
    .await
}

async fn handle_delete_event(
    repo: Arc<dyn Repository>,
    scheduler: Arc<Scheduler>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    if let None = action.value {
        return Err(hyper::StatusCode::BAD_REQUEST);
    }
    if action.value.as_deref().unwrap() == "cancel" {
        return handle_close(&command_action.response_url).await;
    }

    let event_id: u32 = match action.value.clone() {
        Some(id) => match id.parse() {
            Ok(id) => id,
            Err(..) => return Err(hyper::StatusCode::BAD_REQUEST),
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };

    let request = delete_event::Request {
        id: event_id,
        channel: command_action.channel.id.clone(),
    };
    match delete_event::execute(repo.clone(), request).await {
        Ok(..) => (),
        Err(delete_event::Error::NotFound) => return Err(hyper::StatusCode::NOT_FOUND),
        _ => return Err(hyper::StatusCode::INTERNAL_SERVER_ERROR),
    };

    scheduler.remove(event_id).await;

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
        command_action.channel.id.clone(),
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
        command_action.channel.id.clone(),
        command_action.user.id.clone(),
        event_id,
    )
    .await
}

async fn handle_list_event(
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    match action.value.clone() {
        Some(value) if value == "close" => handle_close(&command_action.response_url).await,
        Some(value) if value == "add_event" => {
            handle_create_event(&command_action.response_url).await
        }
        _ => {
            log::trace!("unknown action value for list event: {:?}", action.value);
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    }
}

async fn handle_pick_participant_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    let response_url = command_action.response_url.clone();
    let channel = command_action.channel.id.clone();
    let user = command_action.user.id.clone();
    let event_id = match action.value.clone() {
        Some(value) => match value.parse() {
            Ok(id) => id,
            Err(err) => {
                log::trace!("error retrieving event id from action value: {}", err);
                return Err(hyper::StatusCode::BAD_REQUEST);
            }
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };
    match action.action_id.clone().map(|action_id| {
        action_id
            .clone()
            .trim_start_matches("pick_participant_actions:")
            .to_string()
    }) {
        Some(value) if value == "pick" => {
            handle_skip_pick_event(repo, response_url, channel, user, event_id).await
        }
        Some(value) if value == "repick" => {
            handle_repick_event(repo, response_url, channel, user, event_id).await
        }
        Some(value) if value == "cancel" => {
            handle_cancel_pick(repo, response_url, channel, user, event_id).await
        }
        _ => {
            log::trace!(
                "unknown action value for pick participant event: {:?}",
                action.value
            );
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    }
}

async fn handle_cancel_pick_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
) -> Result<(), hyper::StatusCode> {
    let response_url = command_action.response_url.clone();
    let channel = command_action.channel.id.clone();
    let user = command_action.user.id.clone();
    let event_id = match action.value.clone() {
        Some(value) => match value.parse() {
            Ok(id) => id,
            Err(err) => {
                log::trace!("error retrieving event id from action value: {}", err);
                return Err(hyper::StatusCode::BAD_REQUEST);
            }
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };
    match action.action_id.clone().map(|action_id| {
        action_id
            .clone()
            .trim_start_matches("cancel_pick_actions:")
            .to_string()
    }) {
        Some(value) if value == "pick" => {
            handle_pick_event(repo, response_url, channel, user, event_id).await
        }
        _ => {
            log::trace!(
                "unknown action value for cancel pick event: {:?}",
                action.value
            );
            return Err(hyper::StatusCode::BAD_REQUEST);
        }
    }
}

async fn handle_list_item_event(
    repo: Arc<dyn Repository>,
    action: &Action,
    command_action: &CommandAction,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    let response_url = command_action.response_url.clone();
    let channel = command_action.channel.id.clone();
    let user = command_action.user.id.clone();
    let selected_option = match action.selected_option.clone() {
        Some(option) => match option.value {
            Some(option) => option,
            None => return Err(hyper::StatusCode::BAD_REQUEST),
        },
        None => return Err(hyper::StatusCode::BAD_REQUEST),
    };
    match selected_option.as_str() {
        "pick" => handle_pick_event(repo, response_url, channel, user, event_id).await,
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
        Some(action_id) if action_id == "close" => {
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
    let channel = command_action.channel.id.clone();
    let user = command_action.user.id.clone();
    match action_type.as_str() {
        "pick" => handle_pick_event(repo, response_url, channel, user, event_id).await,
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
        command_action.channel.id.clone(),
        event_id,
    )
    .await
}

async fn handle_pick_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    user: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    if let Some(response) = pick_participant::execute(
        repo.clone(),
        event_id,
        channel,
        user,
        response_url.clone(),
        false,
    )
    .await?
    {
        let body = hyper::Body::from(response.to_string());
        super::send_post(&response_url, body).await.map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    return Ok(());
}

async fn handle_skip_pick_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    user: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    if let Some(response) = pick_participant::execute(
        repo.clone(),
        event_id,
        channel,
        user,
        response_url.clone(),
        true,
    )
    .await?
    {
        let body = hyper::Body::from(response.to_string());
        super::send_post(&response_url, body).await.map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    return Ok(());
}

async fn handle_repick_event(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    user: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    if let Some(response) =
        repick_participant::execute(repo.clone(), event_id, channel, user, response_url.clone())
            .await?
    {
        let body = hyper::Body::from(response.to_string());
        super::send_post(&response_url, body).await.map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    return Ok(());
}

async fn handle_cancel_pick(
    repo: Arc<dyn Repository>,
    response_url: String,
    channel: String,
    user: String,
    event_id: u32,
) -> Result<(), hyper::StatusCode> {
    if let Some(response) =
        cancel_pick::execute(repo.clone(), event_id, channel, user, response_url.clone()).await?
    {
        let body = hyper::Body::from(response.to_string());
        super::send_post(&response_url, body).await.map_err(|err| {
            log::error!("unable to send slack error response: {}", err);
            hyper::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    return Ok(());
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
