use std::sync::Arc;

use serde_json::Value;

use crate::{
    domain::events::{find_event, pick_participant},
    repository::event::Repository,
    slack::helpers::send_post,
    views::pick_participant::{
        view as pick_participant_view, PickParticipantSource, PickParticipantView,
    },
};

pub async fn execute(
    repo: Arc<dyn Repository>,
    event_id: u32,
    channel_id: String,
    user_id: String,
    response_url: String,
    is_skip: bool,
) -> Result<Option<Value>, hyper::StatusCode> {
    let result = match pick_participant::execute(
        repo.clone(),
        pick_participant::Request {
            event: event_id,
            channel: channel_id.clone(),
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
    let event = match find_event::execute(
        repo,
        find_event::Request {
            id: event_id,
            channel: channel_id,
        },
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(match err {
                find_event::Error::NotFound => hyper::StatusCode::NOT_FOUND,
                find_event::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    };
    let left_count =
        event.participants.len() - event.participants.iter().filter(|p| p.picked).count();
    log::trace!("picked new participant: {:?} ({} left)", result, left_count);

    send_post(
        &response_url,
        hyper::Body::from(
            pick_participant_view(PickParticipantView {
                source: if is_skip {
                    PickParticipantSource::Skip
                } else {
                    PickParticipantSource::Pick
                },
                event_id: event_id,
                event_name: event.name.clone(),
                channel_id: event.channel,
                user_picked_id: result.id,
                user_id,
                left_count,
            })
            .to_string(),
        ),
    )
    .await
    .map_err(|err| {
        log::error!("unable to send slack response: {}", err);
        hyper::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    return Ok(None);
}
