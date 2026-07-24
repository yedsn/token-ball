use tauri::{AppHandle, Manager, PhysicalPosition};

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
