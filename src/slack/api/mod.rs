pub mod routes;

mod search_channels;
mod search_events;
mod core;

use core::{ApiError};

#[cfg(test)]
mod test {
    use crate::domain::auth::jwt::Claims;
    use crate::repository::event;
    use log::LevelFilter;
    use std::sync::Arc;
    use super::{search_channels, search_events};

    fn create_claims() -> Claims {
        let access_token = std::env::var("SLACK_ACCESS_TOKEN").expect("SLACK_ACCESS_TOKEN must be set");
        let team_id = std::env::var("SLACK_TEAM_ID").expect("SLACK_TEAM_ID must be set");
        Claims {
            access_token,
            team_id,
            exp: 0,
        }
    }

    async fn create_repository() -> Arc<dyn event::Repository> {
        let db_tool_url =
            std::env::var("DATABASE_TOOL_URL").expect("DATABASE_TOOL_URL must be set");
        let db_tool_name =
            std::env::var("DATABASE_TOOL_NAME").expect("DATABASE_TOOL_NAME must be set");
        Arc::new(event::MongoDbRepository::new(&db_tool_url, &db_tool_name, 10)
            .await
            .unwrap())
    }

    #[tokio::test]
    pub async fn test_search_channels() {
        let claims = create_claims();
        let repository = create_repository().await;
        tracing_subscriber::fmt::init();
        log::set_max_level(LevelFilter::Trace);
        let res = search_channels::execute(claims, repository).await;
        dbg!(&res);
        assert!(res.is_ok());
    }


    #[tokio::test]
    pub async fn test_search_events() {
        let claims = create_claims();
        let repository = create_repository().await;
        // tracing_subscriber::fmt::init();
        // log::set_max_level(LevelFilter::Trace);
        let res = search_events::execute(claims, repository, search_events::ApiRequest {
            channel: "C04PTUB6GF7".to_string(),
        }).await;
        dbg!(&res);
        assert!(res.is_ok());
    }
}

