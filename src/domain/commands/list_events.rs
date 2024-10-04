use std::sync::Arc;

use crate::{
    domain::events::find_all_events, repository::event::Repository, slack::helpers,
    views::list_events,
};

impl From<find_all_events::Response> for list_events::ListEventView {
    fn from(value: find_all_events::Response) -> Self {
        Self {
            id: value.id,
            name: value.name,
            date: helpers::fmt_timestamp(value.timestamp, value.timezone),
            repeat: value.repeat.to_string(),
        }
    }
}

pub async fn execute(
    repo: Arc<dyn Repository>,
    channel: String,
    reached_limit: bool,
) -> Result<serde_json::Value, hyper::StatusCode> {
    let result = match find_all_events::execute(repo, find_all_events::Request::new().with_channel(channel)).await {
        Ok(response) => response.data,
        Err(err) => {
            return Err(match err {
                find_all_events::Error::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    };
    let events = result.into_iter().map(|event| event.into()).collect();

    return Ok(list_events::view(events, reached_limit));
}
