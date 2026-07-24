use std::{collections::HashMap, sync::Arc};

use sqlx::SqlitePool;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: SqlitePool,
    pub sync_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl AppState {
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            sync_locks: Mutex::new(HashMap::new()),
        }
    }
}
