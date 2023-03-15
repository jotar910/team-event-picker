use crate::domain::{update_event::Request, timezone::Timezone};

pub fn mock_update_event_request() -> Request {
    Request {
        id: 0,
        name: "Daily Meeting".to_string(),
        timestamp: 1609459200,
        timezone: Timezone::GMT.into(),
        repeat: "daily".to_string(),
        participants: vec!["João".to_string(), "Joana".to_string()],
        channel: "Channel".to_string(),
    }
}