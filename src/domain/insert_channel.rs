use std::sync::Arc;

use crate::repository::event::{FindError, InsertError, Repository};

use crate::domain::entities::Channel;

pub struct Request {
    pub name: String,
}

pub struct Response {
    pub channel: Channel,
}

pub enum Error {
    Unknown,
}

pub fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    match repo.find_channel_by_name(req.name.clone()) {
        Ok(channel) => return Ok(Response { channel }),
        Err(error) if error != FindError::NotFound => return Err(Error::Unknown),
        _ => (),
    }

    let channel = Channel {
        id: 0,
        name: req.name,
    };

    let channel: Channel = repo.insert_channel(channel).map_err(|error| match error {
        InsertError::Conflict | InsertError::Unknown => Error::Unknown,
    })?;

    Ok(Response { channel })
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::domain::entities::Event;
//     use crate::domain::mocks;
//     use crate::repository::event::InMemoryRepository;

//     #[test]
//     fn it_should_update_participants_for_the_given_event() {
//         let repo = Arc::new(InMemoryRepository::new());

//         let result = repo.insert(mocks::mock_event_creation());

//         match result {
//             Ok(Event { participants, .. }) => assert_eq!(participants, vec![0, 1]),
//             _ => unreachable!(),
//         }

//         // Testing insert_users here ---

//         let req = Request {
//             names: mocks::mock_users_names(),
//         };

//         let result = execute(repo, req);

//         match result {
//             Ok(Response { users }) => assert_eq!(
//                 users.into_iter().map(|user| user.id).collect::<Vec<u32>>(),
//                 vec![2, 3, 1]
//             ),
//             _ => unreachable!(),
//         }
//     }
// }
