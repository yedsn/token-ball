use tauri::AppHandle;

use crate::windows;

#[tauri::command]
pub async fn window_show(app: AppHandle, label: String) -> Result<(), String> {
    windows::show_window(&app, &label);
    Ok(())
}

#[tauri::command]
pub async fn window_hide(app: AppHandle, label: String) -> Result<(), String> {
    windows::hide_window(&app, &label);
    Ok(())
}
