use std::sync::Arc;

use serde_json::Value;

use crate::{
    domain::events::{cancel_pick, find_event},
    repository::event::Repository,
    slack::helpers::send_post,
    views::cancel_pick::{view as cancel_pick_view, CancelPickView},
};

pub async fn execute(
    repo: Arc<dyn Repository>,
    event_id: u32,
    channel_id: String,
    user_id: String,
    response_url: String,
) -> Result<Option<Value>, hyper::StatusCode> {
    let result = match cancel_pick::execute(
        repo.clone(),
        cancel_pick::Request {
            event: event_id,
            channel: channel_id.clone(),
        },
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(match err {
                cancel_pick::Error::Empty => hyper::StatusCode::NOT_ACCEPTABLE,
                cancel_pick::Error::NotFound => hyper::StatusCode::NOT_FOUND,
                cancel_pick::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
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
    log::trace!("cancelled pick: {:?} ({} left)", result, left_count);

    send_post(
        &response_url,
        hyper::Body::from(
            cancel_pick_view(CancelPickView {
                event_id: event_id,
                event_name: event.name.clone(),
                channel_id: event.channel,
                user_id,
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
