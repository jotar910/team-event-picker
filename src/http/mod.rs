use std::sync::Arc;

use axum::{routing, Router, Server};
use hyper::Result;
use log::info;

use super::config::Config;

mod hello_world;

pub struct AppState {
    config: Config,
}

pub async fn serve(config: Config) -> Result<()> {
    let app = Router::new().route("/", routing::get(hello_world::root));

    info!("Listening on port {}", config.port);

    Server::bind(&format!("0.0.0.0:{}", config.port).parse().unwrap())
        .serve(app
            .with_state(Arc::new(AppState { config }))
            .into_make_service())
        .await
}
