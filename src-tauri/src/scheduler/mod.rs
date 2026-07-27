use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::{app_state::AppState, commands::quota::refresh_all_internal, storage::repository};

pub fn start_quota_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        let mut last_refresh = Instant::now();
        loop {
            let interval_secs = repository::load_sync_interval_seconds(&state.db)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(crate::quota::default_sync_interval_seconds);
            let interval = Duration::from_secs(interval_secs);
            let next_due = last_refresh
                .checked_add(interval)
                .unwrap_or_else(Instant::now);
            if Instant::now() >= next_due {
                let _ = refresh_all_internal(&app, &state).await;
                last_refresh = Instant::now();
                continue;
            }
            let sleep_for = next_due
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(60));
            sleep(sleep_for).await;
        }
    });
}
