use std::sync::Arc;

use crate::{repository, scheduler::Scheduler};

#[derive(Clone)]
pub struct AppState {
    pub secret: String,
    pub token: String,
    pub client_id: String,
    pub client_secret: String,
    pub repo: Arc<dyn repository::event::Repository>,
    pub auth_repo: Arc<dyn repository::auth::Repository>,
    pub scheduler: Arc<Scheduler>,
}
