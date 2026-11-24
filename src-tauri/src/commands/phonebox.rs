use crate::{
    app_state::AppState,
    integrations::phonebox::{
        config::{self, PhoneBoxSettings},
        delivery::DeliveryStatus,
    },
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn phonebox_get_settings(
    state: State<'_, Arc<AppState>>,
) -> Result<PhoneBoxSettings, String> {
    config::load(&state.db).await.map_err(Into::into)
}

#[tauri::command]
pub async fn phonebox_save_settings(
    state: State<'_, Arc<AppState>>,
    settings: PhoneBoxSettings,
) -> Result<PhoneBoxSettings, String> {
    let _guard = state.phonebox.settings_lock.lock().await;
    let settings = config::save(&state.db, &settings)
        .await
        .map_err(String::from)?;
    state.phonebox.configure(settings.clone());
    Ok(settings)
}

#[tauri::command]
pub fn phonebox_get_status(state: State<'_, Arc<AppState>>) -> DeliveryStatus {
    state.phonebox.status()
}

#[tauri::command]
pub fn phonebox_push_now(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.phonebox.request_now()
}
