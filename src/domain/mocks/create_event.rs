use std::sync::Arc;

use super::Event;
use crate::domain::create_event::Request;
use crate::domain::timezone::Timezone;
use crate::repository::event::{InMemoryRepository, Repository};

pub fn mock_create_event_request() -> Request {
    Request {
        name: "Daily Meeting".to_string(),
        timestamp: 1609459200,
        timezone: Timezone::GMT.into(),
        repeat: "daily".to_string(),
        participants: vec!["João".to_string(), "Joana".to_string()],
        channel: "Channel".to_string(),
    }
}

pub async fn insert_mock_event(repo: Arc<InMemoryRepository>) -> Event {
    if let Err(..) = repo.insert_channel(super::mock_channel()).await {
        unreachable!("channel must be created for this test")
    }
    if let Err(..) = repo.insert_users(super::mock_participants()).await {
        unreachable!("users must be created for this test")
    }
    match repo.insert_event(super::mock_event()).await {
        Ok(event) => event,
        _ => unreachable!("event must be created for this test"),
    }
}
