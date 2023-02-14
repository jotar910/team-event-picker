use crate::domain::update_participants::Request;

pub fn mock_participants_update() -> Request {
    Request {
        event: 0,
        participants: vec![
            "Francisca".to_string(),
            "Simão".to_string(),
            "Joana".to_string(),
        ],
    }
}