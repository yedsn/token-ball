use std::sync::Arc;

use tauri::State;

use crate::{app_state::AppState, storage::repository};

#[tauri::command]
pub async fn plugin_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<repository::PluginManifest>, String> {
    repository::list_plugins(&state.db)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn plugin_set_enabled(
    state: State<'_, Arc<AppState>>,
    id: String,
    enabled: bool,
) -> Result<Vec<repository::PluginManifest>, String> {
    repository::set_plugin_enabled(&state.db, &id, enabled)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn plugin_add(
    state: State<'_, Arc<AppState>>,
    input: repository::PluginInput,
) -> Result<Vec<repository::PluginManifest>, String> {
    repository::add_plugin(&state.db, input)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn plugin_delete(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Vec<repository::PluginManifest>, String> {
    repository::delete_plugin(&state.db, &id)
        .await
        .map_err(Into::into)
}
