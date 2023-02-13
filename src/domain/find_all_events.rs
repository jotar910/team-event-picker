use std::sync::Arc;

use crate::domain::dtos::ListResponse;
use crate::domain::entities::RepeatPeriod;
use crate::repository::event::{FindAllError, Repository};

pub struct Request {
    pub channel: String,
}

#[derive(Debug, PartialEq)]
pub struct Response {
    pub id: u32,
    pub name: String,
    pub date: String,
    pub repeat: RepeatPeriod,
    pub participants: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Unknown,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<ListResponse<Response>, Error> {
    let events = match repo.find_all(req.channel) {
        Err(err) => {
            return match err {
                FindAllError::Unknown => Err(Error::Unknown),
            }
        }
        Ok(events) => events,
    };
    Ok(ListResponse::new({
        let mut responses = Vec::new();
        for event in events.into_iter() {
            let participants = match repo.find_users(event.participants) {
                Ok(users) => users,
                Err(error) => match error {
                    FindAllError::Unknown => return Err(Error::Unknown),
                },
            };
            let response = Response {
                id: event.id,
                name: event.name,
                date: event.date,
                repeat: event.repeat,
                participants: participants.into_iter().map(|user| user.name).collect(),
            };
            responses.push(response);
        }
        responses
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[test]
    fn it_should_return_all_the_events_for_the_provided_channel() {
        let repo = Arc::new(InMemoryRepository::new());

        if let Err(..) = repo.insert(mocks::mock_event_creation()) {
            unreachable!("event must be created for this test")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += "2";
        mock.channel += "2";
        if let Err(..) = repo.insert(mock) {
            unreachable!("event must be created for this test")
        }

        let mut mock = mocks::mock_event_creation();
        mock.name += "3";
        if let Err(..) = repo.insert(mock) {
            unreachable!("event must be created for this test")
        }

        // Testing find_all_events here --
        let req = Request {
            channel: mocks::mock_event_creation().channel,
        };

        let result = execute(repo, req);

        match result {
            Ok(ListResponse { data }) => {
                assert_eq!(data, mocks::mock_find_all_events_response().data)
            }
            _ => unreachable!(),
        }
    }
}
