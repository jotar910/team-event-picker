use std::sync::Arc;

use crate::domain::entities::ParticipantEdit;
use crate::repository::event::{DeleteError, Repository};

pub struct Request {
    pub event: u32,
    pub participants: Vec<String>,
}

impl From<Request> for ParticipantEdit {
    fn from(value: Request) -> Self {
        ParticipantEdit {
            event: value.event,
            participants: value.participants,
        }
    }
}

pub struct Response {
    pub id: u32,
}

pub enum Error {
    NotFound,
    Unknown,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let event_id = req.event;
    match repo.delete_participants(req.into()) {
        Err(error) => match error {
            DeleteError::NotFound => Err(Error::NotFound),
            DeleteError::Unknown => Err(Error::Unknown),
        },
        Ok(..) => Ok(Response { id: event_id }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[test]
    fn it_should_update_participants() {
        let repo = Arc::new(InMemoryRepository::new());

        match repo.insert(mocks::mock_event_creation()) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0, 1]),
            _ => unreachable!("event must be created for this test"),
        }

        // Testing update_participants here ---

        let req = mocks::mock_participant_update().into();

        let result = execute(repo.clone(), req);

        match result {
            Ok(Response { id, .. }) => assert_eq!(id, 0),
            _ => unreachable!(),
        }

        match repo.find(0) {
            Ok(Event { participants, .. }) => assert_eq!(participants, vec![0]),
            _ => unreachable!(),
        }
    }

    impl From<ParticipantEdit> for Request {
        fn from(value: ParticipantEdit) -> Self {
            Self {
                event: value.event,
                participants: value.participants,
            }
        }
    }
}
