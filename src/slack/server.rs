use std::sync::Arc;

use crate::{
    config::Config,
    domain::{find_all_events_and_dates, pick_auto_participants},
    repository,
    scheduler::{entities::EventSchedule, Scheduler},
    slack::sender,
};

use axum::{Router, Server, middleware, Extension};
use hyper::Result;
use tokio::{join, sync::mpsc, task};

pub async fn serve(config: Config) -> Result<()> {
    let app = Router::new()
        .route(
            "/api/commands",
            axum::routing::post(super::commands::execute),
        )
        .route("/api/actions", axum::routing::post(super::actions::execute))
        .route_layer(middleware::from_fn(super::auth_guard::guard))
        .route("/api/oauth", axum::routing::get(super::oauth::execute));

    log::info!(
        "Connecting to database {}/{}",
        config.database_url,
        config.database_name
    );

    let repo = Arc::new(
        repository::event::MongoDbRepository::new(&config.database_url, &config.database_name, 50)
            .await
            .expect("could not connect to database"),
    );
    let auth_repo = Arc::new(
        repository::auth::MongoDbRepository::new(&config.database_url, "auth", 50)
            .await
            .expect("could not connect to auth database"),
    );
    let (tx, mut rx) = mpsc::channel::<Vec<pick_auto_participants::Pick>>(1);
    let scheduler = Arc::new(Scheduler::new(tx));

    // Initialize server thread.
    let app_scheduler = scheduler.clone();
    let app_repo = repo.clone();
    let app_config = config.clone();
    let server_task = task::spawn(async move {
        log::info!("Listening on port {}", config.port);

        let state = Arc::new(super::AppState {
            secret: app_config.signature,
            token: app_config.bot_token,
            client_id: app_config.client_id,
            client_secret: app_config.client_secret,
            repo: app_repo,
            auth_repo,
            scheduler: app_scheduler,
        });

        if let Err(err) = Server::bind(&format!("0.0.0.0:{}", app_config.port).parse().unwrap())
            .serve(
                app
                .layer(Extension(state.clone()))
                .with_state(state)
                .into_make_service(),
            )
            .await
        {
            log::error!("error initializing server: {}", err);
        }
    });

    // Initialize scheduler thread.
    let app_scheduler = scheduler.clone();
    let app_repo = repo.clone();
    let scheduler_task = task::spawn(async move {
        log::info!("Scheduler is running");
        app_scheduler.start(app_repo).await;
    });

    // Initialize auto-picker listener thread.
    let auto_picker_task = task::spawn(async move {
        while let Some(picks) = rx.recv().await {
            sender::post_picks(&config.bot_token, picks).await;
        }
    });

    log::info!("Fetching events to fill up scheduler");
    match find_all_events_and_dates::execute(repo).await {
        Ok(events) => {
            for event in events.data.into_iter() {
                scheduler
                    .insert(EventSchedule {
                        id: event.id,
                        timestamp: event.timestamp,
                        timezone: event.timezone,
                        repeat: event.repeat,
                    })
                    .await;
            }
        }
        Err(err) => {
            log::error!("could no fetch events for scheduling: {:?}", err);
        }
    };

    let (server_result, scheduler_result, auto_picker_result) =
        join!(server_task, scheduler_task, auto_picker_task);

    scheduler_result.expect("failed running scheduler");
    auto_picker_result.expect("failed running auto-picker");
    Ok(server_result.expect("failed running server"))
}
