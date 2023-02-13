use crate::domain::{dtos::ListResponse, find_all_events::Response};

pub fn mock_find_all_events_response() -> ListResponse<Response> {
    let event_creation_0 = super::mock_event_creation();
    let event_creation_1 = super::mock_event_creation();
    ListResponse::new(vec![
        Response {
            id: 0,
            name: event_creation_0.name,
            date: event_creation_0.date,
            repeat: event_creation_0.repeat,
            participants: event_creation_0.participants,
        },
        Response {
            id: 2,
            name: event_creation_1.name + "3",
            date: event_creation_1.date,
            repeat: event_creation_1.repeat,
            participants: event_creation_1.participants,
        },
    ])
}
