use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;

use crate::repository::errors::{FindAllError, InsertError};
use crate::repository::event::Repository;

use crate::domain::entities::User;

pub struct Request {
    pub names: Vec<String>,
}

pub struct Response {
    pub users: Vec<User>,
}

pub enum Error {
    Unknown,
}

pub async fn execute(repo: Arc<dyn Repository>, req: Request) -> Result<Response, Error> {
    let mut users_map: HashMap<String, Option<User>> =
        req.names.iter().map(|name| (name.clone(), None)).collect();

    fill_with_existing_users(repo.clone(), &req.names, &mut users_map).await?;

    let mut add_users: Vec<User> = vec![];
    for name in req.names.iter().unique() {
        let user = users_map.get(name).unwrap();
        if let None = user {
            add_users.push(User {
                id: 0,
                name: name.to_string(),
            })
        }
    }

    if add_users.len() > 0 {
        let add_users: Vec<User> =
            repo.insert_users(add_users)
                .await
                .map_err(|error| match error {
                    InsertError::Conflict | InsertError::Unknown => Error::Unknown,
                })?;

        for existing_user in add_users.into_iter() {
            users_map.insert(existing_user.name.clone(), Some(existing_user));
        }
    }

    Ok(Response {
        users: req
            .names
            .into_iter()
            .map(|name| users_map[&name].as_ref().unwrap().clone())
            .collect(),
    })
}

async fn fill_with_existing_users<'a>(
    repo: Arc<dyn Repository>,
    names: &'a Vec<String>,
    users_to_fill: &'a mut HashMap<String, Option<User>>,
) -> Result<(), Error> {
    let users = repo
        .find_users_by_name(names.clone())
        .await
        .map_err(|error| match error {
            FindAllError::Unknown => Error::Unknown,
        })?;

    for existing_user in users {
        if !users_to_fill.contains_key(&existing_user.name) {
            continue;
        }
        users_to_fill.insert(existing_user.name.clone(), Some(existing_user.clone()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mocks;
    use crate::repository::event::InMemoryRepository;

    #[tokio::test]
    async fn it_should_update_participants_for_the_given_event() {
        let repo = Arc::new(InMemoryRepository::new());

        let result = mocks::insert_mock_event(repo.clone()).await;

        assert_eq!(result.participants, vec![0, 1]);

        // Testing insert_users here ---

        let req = Request {
            names: mocks::mock_users_names(),
        };

        let result = execute(repo, req).await;

        match result {
            Ok(Response { users }) => assert_eq!(
                users.into_iter().map(|user| user.id).collect::<Vec<u32>>(),
                vec![2, 3, 1]
            ),
            _ => unreachable!(),
        }
    }
}
