use tauri::{AppHandle, Manager, PhysicalPosition};

use crate::events;

pub fn show_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        if label == "hover" {
            if let Some(orb) = app.get_webview_window("orb") {
                if let Ok(position) = orb.outer_position() {
                    let _ = window.set_position(PhysicalPosition::new(position.x + 96, position.y));
                }
            }
        }
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
}

pub fn open_main_overview(app: &AppHandle) {
    hide_window(app, "hover");
    show_window(app, "main");
    events::emit_show_overview(app);
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        events::emit_show_overview(&app_handle);
    });
}
