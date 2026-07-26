use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

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

#[tauri::command]
pub async fn window_open_main_overview(app: AppHandle) -> Result<(), String> {
    windows::open_main_overview(&app);
    Ok(())
}

#[tauri::command]
pub async fn window_minimize_main(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn window_toggle_main_maximize(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_maximized().unwrap_or(false) {
            window.unmaximize().map_err(|error| error.to_string())?;
        } else {
            window.maximize().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn window_close_main(app: AppHandle) -> Result<(), String> {
    windows::handle_main_close(&app);
    Ok(())
}
