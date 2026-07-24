use std::{sync::Arc, time::Duration};

use tauri::{AppHandle, Manager};
use tokio::time::interval;

use crate::{app_state::AppState, commands::quota::refresh_all_internal};

pub fn start_quota_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(180));
        loop {
            ticker.tick().await;
            let state = app.state::<Arc<AppState>>();
            let _ = refresh_all_internal(&app, &state).await;
        }
    });
}
