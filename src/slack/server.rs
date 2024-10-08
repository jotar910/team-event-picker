use std::sync::Arc;

use axum::extract::MatchedPath;
use axum::{middleware, Extension, Router, Server};
use hyper::{Body, Request, Result};
use tokio::{join, sync::mpsc, task};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;

use crate::{
    config::Config,
    domain::events::{find_all_events_and_dates, pick_auto_participants},
    repository,
    scheduler::{entities::EventSchedule, Scheduler},
    slack::{sender, state::AppConfigs},
};

pub async fn serve(config: Config) -> Result<()> {
    let app = Router::new()
        .route(
            "/api/commands",
            axum::routing::post(super::commands::execute),
        )
        .route("/api/actions", axum::routing::post(super::actions::execute))
        .route_layer(middleware::from_fn(super::guard::validate))
        .route("/api/oauth", axum::routing::get(super::oauth::execute))
        .route("/health", axum::routing::get(health))
        .layer(
            TraceLayer::new_for_http()
                // Create our own span for the request and include the matched path. The matched
                // path is useful for figuring out which handler the request was routed to.
                .make_span_with(|req: &Request<Body>| {
                    let method = req.method();
                    let uri = req.uri();

                    // axum automatically adds this extension.
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|matched_path| matched_path.as_str());

                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                // By default `TraceLayer` will log 5xx responses but we're doing our specific
                // logging of errors so disable that
                .on_failure(()),
        );

    log::info!(
        "Connecting to database {}/{}",
        config.database_tool_url,
        config.database_tool_name
    );

    let event_repo = Arc::new(
        repository::event::MongoDbRepository::new(
            &config.database_tool_url,
            &config.database_tool_name,
            50,
        )
        .await
        .expect("could not connect to tool database"),
    );

    log::info!(
        "Connecting to database {}/{}",
        config.database_auth_url,
        config.database_auth_name
    );

    let auth_repo = Arc::new(
        repository::auth::MongoDbRepository::new(
            &config.database_auth_url,
            &config.database_auth_name,
            50,
        )
        .await
        .expect("could not connect to auth database"),
    );
    let (tx, mut rx) = mpsc::channel::<Vec<pick_auto_participants::Pick>>(1);
    let scheduler = Arc::new(Scheduler::new(tx));

    // Initialize server thread.
    let app_scheduler = scheduler.clone();
    let app_event_repo = event_repo.clone();
    let app_auth_repo = auth_repo.clone();
    let app_config = config.clone();
    let server_task = task::spawn(async move {
        log::info!("Listening on port {}", config.port);

        let state = Arc::new(super::AppState {
            configs: Arc::new(AppConfigs {
                app_id: app_config.app_id,
                secret: app_config.signature,
                client_id: app_config.client_id,
                client_secret: app_config.client_secret,
                max_events: app_config.max_events,
            }),
            event_repo: app_event_repo,
            auth_repo: app_auth_repo,
            scheduler: app_scheduler,
        });

        if let Err(err) = Server::bind(&format!("0.0.0.0:{}", app_config.port).parse().unwrap())
            .serve(
                app.layer(Extension(state.clone()))
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
    let app_event_repo = event_repo.clone();
    let scheduler_task = task::spawn(async move {
        log::info!("Scheduler is running");
        app_scheduler.start(app_event_repo, auth_repo).await;
    });

    // Initialize auto-picker listener thread.
    let auto_picker_task = task::spawn(async move {
        while let Some(picks) = rx.recv().await {
            sender::post_picks(picks).await;
        }
    });

    log::info!("Fetching events to fill up scheduler");
    match find_all_events_and_dates::execute(event_repo).await {
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

async fn health() -> String {
    String::from("OK")
}
