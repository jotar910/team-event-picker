use std::sync::Arc;

use crate::repository::event::Repository;

pub struct AppState {
    pub secret: String,
    pub repo: Arc<dyn Repository>,
}
