use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Rect, WebviewWindow};

use crate::{app_state::AppState, events, storage::repository};

pub const MAIN_WINDOW_STATE_KEY: &str = "window.main.state";
const DEFAULT_HOVER_WIDTH: i32 = 520;
const DEFAULT_HOVER_HEIGHT: i32 = 470;

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
                if let (Ok(orb_position), Ok(orb_size), Ok(hover_size)) =
                    (orb.outer_position(), orb.outer_size(), window.outer_size())
                {
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
        if label != "hover" {
            let _ = window.set_focus();
        }
    }
}

pub fn handle_main_close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        save_main_window_state(app, &window);
        let _ = window.hide();
    }
}

pub fn save_main_window_state(app: &AppHandle, window: &WebviewWindow) {
    let Some(saved) = main_window_state(window) else {
        return;
    };
    let Ok(value) = serde_json::to_string(&saved) else {
        return;
    };
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
    let Some(saved) = saved else {
        return;
    };
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

pub fn show_hover_near_tray(app: &AppHandle, rect: Rect, fallback: PhysicalPosition<f64>) {
    let Some(window) = app.get_webview_window("hover") else {
        return;
    };
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let (anchor, anchor_width) = tray_hover_anchor(rect, fallback, scale_factor);
    show_hover_window_near(&window, anchor, anchor_width);
}

fn tray_hover_anchor(
    rect: Rect,
    fallback: PhysicalPosition<f64>,
    scale_factor: f64,
) -> (PhysicalPosition<i32>, i32) {
    let rect_size = rect.size.to_physical::<i32>(scale_factor);
    if rect_size.width > 0 && rect_size.height > 0 {
        (
            rect.position.to_physical::<i32>(scale_factor),
            rect_size.width,
        )
    } else {
        (fallback.cast::<i32>(), 0)
    }
}

fn show_hover_window_near(
    window: &WebviewWindow,
    anchor: PhysicalPosition<i32>,
    anchor_width: i32,
) {
    let (hover_width, hover_height) = window
        .outer_size()
        .map(|size| (size.width as i32, size.height as i32))
        .unwrap_or((DEFAULT_HOVER_WIDTH, DEFAULT_HOVER_HEIGHT));
    let monitor = window
        .monitor_from_point(anchor.x as f64, anchor.y as f64)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());
    let position = hover_position_near(anchor, anchor_width, hover_width, hover_height, 8, monitor);
    let _ = window.set_position(position);
    let _ = window.show();
    let _ = window.emit("hover://orb-enter", ());
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

/// 打开主窗口并跳转到「关于/检查更新」面板，供托盘菜单调用。
pub fn open_main_update(app: &AppHandle) {
    hide_window(app, "hover");
    show_window(app, "main");
    events::emit_show_update(app);
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        events::emit_show_update(&app_handle);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::{LogicalPosition, LogicalSize, PhysicalPosition, Position, Rect, Size};

    #[test]
    fn tray_hover_anchor_uses_rect_when_available() {
        let rect = Rect {
            position: Position::Logical(LogicalPosition::new(100.0, 200.0)),
            size: Size::Logical(LogicalSize::new(24.0, 24.0)),
        };
        let fallback = PhysicalPosition::new(500.0, 600.0);

        let (anchor, width) = tray_hover_anchor(rect, fallback, 1.5);

        assert_eq!(anchor, PhysicalPosition::new(150, 300));
        assert_eq!(width, 36);
    }

    #[test]
    fn tray_hover_anchor_falls_back_when_rect_is_empty() {
        let rect = Rect {
            position: Position::Logical(LogicalPosition::new(100.0, 200.0)),
            size: Size::Logical(LogicalSize::new(0.0, 0.0)),
        };
        let fallback = PhysicalPosition::new(500.0, 600.0);

        let (anchor, width) = tray_hover_anchor(rect, fallback, 1.5);

        assert_eq!(anchor, PhysicalPosition::new(500, 600));
        assert_eq!(width, 0);
    }
}
