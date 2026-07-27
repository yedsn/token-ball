use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::{app_state::AppState, commands::quota::refresh_all_internal, storage::repository};

pub fn start_quota_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        let mut last_refresh = Instant::now();
        let mut scheduled_interval = Duration::from_secs(crate::quota::default_sync_interval_seconds());
        let mut last_config: Option<(u64, bool, u64, u64)> = None;
        loop {
            let settings = repository::load_display_settings(&state.db)
                .await
                .ok()
                .unwrap_or_default();
            let config = sync_config(&settings);
            if last_config != Some(config) {
                scheduled_interval = next_scheduled_interval(&settings);
                last_config = Some(config);
            }
            let next_due = last_refresh
                .checked_add(scheduled_interval)
                .unwrap_or_else(Instant::now);
            if Instant::now() >= next_due {
                let _ = refresh_all_internal(&app, &state).await;
                last_refresh = Instant::now();
                scheduled_interval = next_scheduled_interval(&settings);
                last_config = Some(config);
                continue;
            }
            let sleep_for = next_due
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(60));
            sleep(sleep_for).await;
        }
    });
}

fn sync_config(settings: &crate::quota::DisplaySettings) -> (u64, bool, u64, u64) {
    (
        settings.sync_interval_seconds,
        settings.random_sync_delay_enabled,
        settings.random_sync_delay_min_seconds,
        settings.random_sync_delay_max_seconds,
    )
}

fn next_scheduled_interval(settings: &crate::quota::DisplaySettings) -> Duration {
    Duration::from_secs(settings.sync_interval_seconds) + random_sync_delay(settings)
}

fn random_sync_delay(settings: &crate::quota::DisplaySettings) -> Duration {
    if !settings.random_sync_delay_enabled {
        return Duration::ZERO;
    }
    let min = settings.random_sync_delay_min_seconds;
    let max = settings.random_sync_delay_max_seconds;
    if max <= min {
        return Duration::from_secs(min);
    }
    let range = max - min + 1;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    Duration::from_secs(min + nanos % range)
}
