use std::sync::Arc;

use crate::{config::Config, repository::event::MongoDbRepository};

use axum::{Router, Server};
use hyper::Result;

pub async fn serve(config: Config) -> Result<()> {
    let app = Router::new()
        .route(
            "/api/commands",
            axum::routing::post(super::commands::execute),
        )
        .route("/api/actions", axum::routing::post(super::actions::execute));

    log::info!(
        "Connecting to database {}/{}",
        config.database_url,
        config.database_name
    );

    let repo = Arc::new(
        MongoDbRepository::new(&config.database_url, &config.database_name, 50)
            .await
            .expect("could not connect to database"),
    );

    log::info!("Listening on port {}", config.port);

    Server::bind(&format!("0.0.0.0:{}", config.port).parse().unwrap())
        .serve(
            app.with_state(Arc::new(super::AppState {
                secret: config.signature,
                repo: repo.clone(),
            }))
            .into_make_service(),
        )
        .await
}
