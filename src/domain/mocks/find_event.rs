use super::User;
use crate::domain::{entities::Channel, find_event::Response};

pub fn mock_find_event_response() -> Response {
    let event_creation = super::mock_event_creation();
    Response {
        id: 0,
        name: event_creation.name,
        date: event_creation.date,
        repeat: event_creation.repeat,
        participants: event_creation
            .participants
            .iter()
            .enumerate()
            .map(|(i, participant)| User {
                id: i as u32,
                name: participant.to_string(),
            })
            .collect(),
        channel: Channel {
            id: 0,
            name: event_creation.channel,
        },
    }
}
