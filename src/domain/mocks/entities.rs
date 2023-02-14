use super::{Channel, Event, EventCreation, RepeatPeriod, User};

pub fn mock_event() -> Event {
    Event {
        id: 0,
        name: "Daily Meeting".to_string(),
        date: "2001-01-01T01:00:00.000Z".to_string(),
        repeat: RepeatPeriod::Daily,
        participants: vec![0, 1],
        channel: 0,
        prev_pick: 0,
        cur_pick: 0,
        deleted: false,
    }
}

pub fn mock_event_creation() -> EventCreation {
    EventCreation {
        name: "Daily Meeting".to_string(),
        date: "2001-01-01T01:00:00.000Z".to_string(),
        repeat: RepeatPeriod::Daily,
        participants: vec!["João".to_string(), "Joana".to_string()],
        channel: "Channel".to_string(),
    }
}

pub fn mock_channel() -> Channel {
    Channel {
        id: 0,
        name: "Channel".to_string(),
    }
}

pub fn mock_participants() -> Vec<User> {
    vec![
        User {
            id: 0,
            name: "João".to_string(),
        },
        User {
            id: 1,
            name: "Joana".to_string(),
        },
    ]
}

pub fn mock_users_names() -> Vec<String> {
    vec![
        "Francisca".to_string(),
        "Simão".to_string(),
        "Joana".to_string(),
    ]
}
