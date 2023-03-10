use std::sync::Arc;

use crate::{repository::event::Repository, scheduler::Scheduler};

pub struct AppState {
    pub secret: String,
    pub token: String,
    pub repo: Arc<dyn Repository>,
    pub scheduler: Arc<Scheduler>,
}
