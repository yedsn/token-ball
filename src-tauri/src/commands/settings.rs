use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{app_state::AppState, events, quota::DisplaySettings, storage::repository};

#[tauri::command]
pub async fn settings_get_display(
    state: State<'_, Arc<AppState>>,
) -> Result<DisplaySettings, String> {
    repository::load_display_settings(&state.db)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn settings_save_display(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: DisplaySettings,
) -> Result<DisplaySettings, String> {
    let settings = repository::save_display_settings(&state.db, &settings)
        .await
        .map_err::<String, _>(Into::into)?;
    events::emit_display_settings_updated(&app, &settings);
    crate::tray::update_tray_from_storage(&app).await;
    Ok(settings)
}
