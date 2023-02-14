use crate::domain::{dtos::ListResponse, find_all_events::Response};

pub fn mock_find_all_events_response() -> ListResponse<Response> {
    let event_creation_0 = super::mock_event();
    let event_creation_1 = super::mock_event();
    ListResponse::new(vec![
        Response {
            id: 0,
            name: event_creation_0.name,
            date: event_creation_0.date,
            repeat: event_creation_0.repeat,
            participants: super::mock_participants().into_iter().map(|p| p.name).collect(),
        },
        Response {
            id: 2,
            name: event_creation_1.name + "3",
            date: event_creation_1.date,
            repeat: event_creation_1.repeat,
            participants: super::mock_participants().into_iter().map(|p| p.name).collect(),
        },
    ])
}
