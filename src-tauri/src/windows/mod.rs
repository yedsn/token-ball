use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

use crate::{app_state::AppState, events, storage::repository};

pub const MAIN_WINDOW_STATE_KEY: &str = "window.main.state";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainWindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
    pub fullscreen: bool,
}

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
        if label == "main" {
            restore_main_window_state(app, &window);
        }
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn handle_main_close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        save_main_window_state(app, &window);
        let _ = window.hide();
    }
}

pub fn save_main_window_state(app: &AppHandle, window: &WebviewWindow) {
    let Some(saved) = main_window_state(window) else { return; };
    let Ok(value) = serde_json::to_string(&saved) else { return; };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        if let Ok(mut cached) = state.main_window_state.write() {
            *cached = Some(saved);
        }
        let _ = repository::set_setting(&state.db, MAIN_WINDOW_STATE_KEY, &value).await;
    });
}

pub fn main_window_state_json(window: &WebviewWindow) -> Option<String> {
    serde_json::to_string(&main_window_state(window)?).ok()
}

pub fn parse_main_window_state(value: &str) -> Option<MainWindowState> {
    serde_json::from_str::<MainWindowState>(value).ok()
}

fn main_window_state(window: &WebviewWindow) -> Option<MainWindowState> {
    let size = window.outer_size().ok()?;
    let position = window.outer_position().ok()?;
    Some(MainWindowState {
        width: size.width,
        height: size.height,
        x: position.x,
        y: position.y,
        maximized: window.is_maximized().unwrap_or(false),
        fullscreen: window.is_fullscreen().unwrap_or(false),
    })
}

fn restore_main_window_state(app: &AppHandle, window: &WebviewWindow) {
    let state = app.state::<Arc<AppState>>();
    let saved = state
        .main_window_state
        .read()
        .ok()
        .and_then(|saved| saved.clone());
    let Some(saved) = saved else { return; };
    let _ = window.unmaximize();
    let _ = window.set_fullscreen(false);
    let _ = window.set_size(tauri::PhysicalSize::new(saved.width, saved.height));
    let _ = window.set_position(PhysicalPosition::new(saved.x, saved.y));
    if saved.maximized {
        let _ = window.maximize();
    }
    if saved.fullscreen {
        let _ = window.set_fullscreen(true);
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
            let _ = window.emit("hover://orb-enter", ());
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
