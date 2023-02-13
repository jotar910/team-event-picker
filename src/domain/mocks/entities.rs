use super::{Channel, EventCreation, ParticipantUpdate, RepeatPeriod};

pub fn mock_event_creation() -> EventCreation {
    EventCreation {
        name: "Daily Meeting".to_string(),
        date: "2001-01-01T01:00:00.000Z".to_string(),
        repeat: RepeatPeriod::Daily,
        participants: vec!["João".to_string(), "Joana".to_string()],
        channel: "Channel".to_string(),
    }
}

pub fn mock_participant_update() -> ParticipantUpdate {
    ParticipantUpdate {
        event: 0,
        participants: vec![
            "Francisca".to_string(),
            "Simão".to_string(),
            "Joana".to_string(),
        ],
    }
}

pub fn mock_channel() -> Channel {
    Channel {
        id: 0,
        name: "Channel".to_string(),
    }
}
