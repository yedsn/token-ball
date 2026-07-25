use std::{sync::Arc, time::Duration};

use tauri::{AppHandle, Manager};
use tokio::time::interval;

use crate::{app_state::AppState, commands::quota::refresh_all_internal, storage::repository};

pub fn start_quota_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        let interval_secs = repository::get_setting(&state.db, "sync.intervalSeconds")
            .await
            .ok()
            .flatten()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(3600);
        let mut ticker = interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            let _ = refresh_all_internal(&app, &state).await;
        }
    });
}
