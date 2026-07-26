use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::windows::MainWindowState;

pub struct AppState {
    pub db: SqlitePool,
    pub sync_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    pub main_window_state: RwLock<Option<MainWindowState>>,
}

impl AppState {
    pub fn new(db: SqlitePool, main_window_state: Option<MainWindowState>) -> Self {
        Self {
            db,
            sync_locks: Mutex::new(HashMap::new()),
            main_window_state: RwLock::new(main_window_state),
        }
    }
}
