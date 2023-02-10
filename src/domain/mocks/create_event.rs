use super::Request;

pub fn mock_request() -> Request {
    Request {
        name: "Daily Meeting".to_string(),
        date: "2001-01-01T01:00:00.000Z".to_string(),
        repeat: "daily".to_string(),
        participants: vec!["João".to_string(), "Joana".to_string()],
    }
}