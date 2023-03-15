use crate::domain::{dtos::ListResponse, find_all_events::Response};

pub fn mock_find_all_events_response() -> ListResponse<Response> {
    let event_creation_0 = super::mock_event();
    let event_creation_1 = super::mock_event();
    ListResponse::new(vec![
        Response {
            id: 0,
            name: event_creation_0.name,
            timestamp: event_creation_0.timestamp,
            timezone: event_creation_0.timezone,
            repeat: event_creation_0.repeat,
            participants: super::mock_participants().into_iter().map(|p| p.name).collect(),
        },
        Response {
            id: 2,
            name: event_creation_1.name + "3",
            timestamp: event_creation_1.timestamp,
            timezone: event_creation_1.timezone,
            repeat: event_creation_1.repeat,
            participants: super::mock_participants().into_iter().map(|p| p.name).collect(),
        },
    ])
}
