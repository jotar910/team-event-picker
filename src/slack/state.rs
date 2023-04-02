use std::sync::Arc;

use crate::{repository, scheduler::Scheduler};

#[derive(Clone)]
pub struct AppState {
    pub event_repo: Arc<dyn repository::event::Repository>,
    pub auth_repo: Arc<dyn repository::auth::Repository>,
    pub scheduler: Arc<Scheduler>,
    pub configs: Arc<AppConfigs>,
}


pub struct AppConfigs {
    pub secret: String,
    pub client_id: String,
    pub client_secret: String,
    pub max_events: u32,
}