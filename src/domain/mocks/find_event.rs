use crate::domain::find_event::Response;

pub fn mock_find_event_response() -> Response {
    let event_creation = super::mock_event();
    Response {
        id: 0,
        name: event_creation.name,
        date: event_creation.date,
        repeat: event_creation.repeat,
        participants: super::mock_participants(),
        channel: super::mock_channel(),
    }
}
