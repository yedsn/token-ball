use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{app_state::AppState, storage::repository, windows};

#[tauri::command]
pub async fn window_show(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    label: String,
) -> Result<(), String> {
    if label == "orb" {
        repository::set_bool_setting(&state.db, "orb.visible", true)
            .await
            .map_err::<String, _>(Into::into)?;
    }
    windows::show_window(&app, &label);
    Ok(())
}

#[tauri::command]
pub async fn window_hide(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    label: String,
) -> Result<(), String> {
    if label == "orb" {
        repository::set_bool_setting(&state.db, "orb.visible", false)
            .await
            .map_err::<String, _>(Into::into)?;
    }
    windows::hide_window(&app, &label);
    Ok(())
}

#[tauri::command]
pub async fn orb_get_visible(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    repository::get_bool_setting(&state.db, "orb.visible", true)
        .await
        .map_err(Into::into)
}
