use tauri::{AppHandle, Manager, PhysicalPosition};

use crate::events;

pub fn show_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        if label == "hover" {
            if let Some(orb) = app.get_webview_window("orb") {
                if let (Ok(orb_position), Ok(orb_size), Ok(hover_size)) = (
                    orb.outer_position(),
                    orb.outer_size(),
                    window.outer_size(),
                ) {
                    let position = hover_position_near(
                        orb_position,
                        orb_size.width as i32,
                        hover_size.width as i32,
                        hover_size.height as i32,
                        12,
                        orb.current_monitor().ok().flatten(),
                    );
                    let _ = window.set_position(position);
                }
            }
        }
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn show_hover_at(app: &AppHandle, anchor: PhysicalPosition<i32>) {
    if let Some(window) = app.get_webview_window("hover") {
        if let Ok(hover_size) = window.outer_size() {
            let position = hover_position_near(
                anchor,
                0,
                hover_size.width as i32,
                hover_size.height as i32,
                8,
                window.current_monitor().ok().flatten(),
            );
            let _ = window.set_position(position);
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn hover_position_near(
    anchor: PhysicalPosition<i32>,
    anchor_width: i32,
    hover_width: i32,
    hover_height: i32,
    gap: i32,
    monitor: Option<tauri::Monitor>,
) -> PhysicalPosition<i32> {
    let right_x = anchor.x + anchor_width + gap;
    let left_x = anchor.x - hover_width - gap;
    let mut x = right_x;
    let mut y = anchor.y;

    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        let area_x = area.position.x;
        let area_y = area.position.y;
        let area_right = area_x + area.size.width as i32;
        let area_bottom = area_y + area.size.height as i32;

        if right_x + hover_width > area_right && left_x >= area_x {
            x = left_x;
        }
        if x < area_x {
            x = area_x + gap;
        }
        if x + hover_width > area_right {
            x = area_right - hover_width - gap;
        }
        if y + hover_height > area_bottom {
            y = area_bottom - hover_height - gap;
        }
        if y < area_y {
            y = area_y + gap;
        }
    }

    PhysicalPosition::new(x, y)
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
