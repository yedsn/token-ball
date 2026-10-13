use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Rect, WebviewWindow};

use crate::{app_state::AppState, debug_log, events, storage::repository, tray};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};

pub const MAIN_WINDOW_STATE_KEY: &str = "window.main.state";
const MAIN_WINDOW_DEFAULT_WIDTH: u32 = 1080;
const MAIN_WINDOW_DEFAULT_HEIGHT: u32 = 720;
const MAIN_WINDOW_MIN_WIDTH: u32 = 900;
const MAIN_WINDOW_MIN_HEIGHT: u32 = 620;
const DEFAULT_HOVER_WIDTH: i32 = 520;
const DEFAULT_HOVER_HEIGHT: i32 = 470;
const WINDOW_STATE_SAVE_DELAY_MS: u64 = 180;
const HOVER_TRAY_GAP: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl PhysicalBounds {
    pub fn contains(self, point: PhysicalPosition<i32>) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }
}

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
    debug_log::line(format!("show_window: {label}"));
    if let Some(window) = app.get_webview_window(label) {
        if label == "hover" {
            if let Some(orb) = app.get_webview_window("orb") {
                if let (Ok(orb_position), Ok(orb_size), Ok(hover_size)) =
                    (orb.outer_position(), orb.outer_size(), window.outer_size())
                {
                    let anchor = PhysicalBounds {
                        x: orb_position.x,
                        y: orb_position.y,
                        width: orb_size.width as i32,
                        height: orb_size.height as i32,
                    };
                    let position = hover_position_near(
                        anchor,
                        hover_size,
                        12,
                        orb.current_monitor().ok().flatten(),
                    );
                    let _ = window.set_position(position);
                }
            }
        }
        reveal_webview_window(&window, label == "main");
    }
}

pub fn handle_main_close(app: &AppHandle) {
    debug_log::line("handle_main_close");
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
    let state = app.state::<Arc<AppState>>();
    if let Ok(mut cached) = state.main_window_state.write() {
        *cached = Some(saved);
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        let _ = repository::set_setting(&state.db, MAIN_WINDOW_STATE_KEY, &value).await;
    });
}

pub fn save_main_window_state_delayed(app: &AppHandle, window: &WebviewWindow) {
    let app = app.clone();
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(WINDOW_STATE_SAVE_DELAY_MS)).await;
        save_main_window_state(&app, &window);
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
    Some(normalize_main_window_state(MainWindowState {
        width: size.width,
        height: size.height,
        x: position.x,
        y: position.y,
        maximized: window.is_maximized().unwrap_or(false),
        fullscreen: window.is_fullscreen().unwrap_or(false),
    }))
}

fn normalize_main_window_state(mut state: MainWindowState) -> MainWindowState {
    state.width = state.width.max(MAIN_WINDOW_MIN_WIDTH);
    state.height = state.height.max(MAIN_WINDOW_MIN_HEIGHT);
    state
}

pub fn apply_cached_main_window_state(app: &AppHandle, window: &WebviewWindow) {
    let state = app.state::<Arc<AppState>>();
    let saved = state
        .main_window_state
        .read()
        .ok()
        .and_then(|saved| saved.clone());
    let Some(saved) = saved else {
        return;
    };
    let saved = normalize_main_window_state(saved);
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

pub fn reset_main_window(app: &AppHandle) {
    hide_window(app, "hover");
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let _ = window.unminimize();
    let _ = window.unmaximize();
    let _ = window.set_fullscreen(false);
    let _ = window.set_size(tauri::PhysicalSize::new(
        MAIN_WINDOW_DEFAULT_WIDTH,
        MAIN_WINDOW_DEFAULT_HEIGHT,
    ));
    let _ = window.center();
    reveal_webview_window(&window, true);
    save_main_window_state(app, &window);
}

pub fn show_hover_near_tray(
    app: &AppHandle,
    rect: Rect,
    fallback: PhysicalPosition<f64>,
) -> Option<(PhysicalBounds, PhysicalBounds)> {
    debug_log::line("show_hover_near_tray");
    let Some(window) = app.get_webview_window("hover") else {
        return None;
    };
    let monitor = window
        .monitor_from_point(fallback.x, fallback.y)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());
    let target_scale_factor = monitor
        .as_ref()
        .map(|monitor| monitor.scale_factor())
        .unwrap_or_else(|| window.scale_factor().unwrap_or(1.0));
    let tray_bounds = tray_hover_bounds(rect, fallback, target_scale_factor);
    let hover_size = hover_physical_size(&window, target_scale_factor);
    let position = hover_position_near(tray_bounds, hover_size, HOVER_TRAY_GAP, monitor);

    debug_log::line(format!("hover set_position: begin {position:?}"));
    let position_result = window.set_position(position);
    debug_log::line(format!(
        "hover set_position: end result={position_result:?}"
    ));
    if position_result.is_err() {
        return None;
    }

    debug_log::line("hover unminimize: begin");
    let unminimize_result = window.unminimize();
    debug_log::line(format!(
        "hover unminimize: end result={unminimize_result:?}"
    ));
    debug_log::line("hover show: begin");
    let show_result = window.show();
    debug_log::line(format!("hover show: end result={show_result:?}"));
    if show_result.is_err() {
        return None;
    }

    let hover_bounds = window_bounds(&window).unwrap_or(PhysicalBounds {
        x: position.x,
        y: position.y,
        width: hover_size.width as i32,
        height: hover_size.height as i32,
    });
    let _ = window.emit("hover://orb-enter", ());
    Some((tray_bounds, hover_bounds))
}

pub fn tray_bounds_for_event(
    app: &AppHandle,
    rect: Rect,
    fallback: PhysicalPosition<f64>,
) -> PhysicalBounds {
    let scale_factor = app
        .get_webview_window("hover")
        .and_then(|window| {
            window
                .monitor_from_point(fallback.x, fallback.y)
                .ok()
                .flatten()
                .map(|monitor| monitor.scale_factor())
                .or_else(|| window.scale_factor().ok())
        })
        .unwrap_or(1.0);
    tray_hover_bounds(rect, fallback, scale_factor)
}

fn tray_hover_bounds(
    rect: Rect,
    fallback: PhysicalPosition<f64>,
    scale_factor: f64,
) -> PhysicalBounds {
    let rect_size = rect.size.to_physical::<i32>(scale_factor);
    if rect_size.width > 0 && rect_size.height > 0 {
        let position = rect.position.to_physical::<i32>(scale_factor);
        PhysicalBounds {
            x: position.x,
            y: position.y,
            width: rect_size.width,
            height: rect_size.height,
        }
    } else {
        let fallback = fallback.cast::<i32>();
        PhysicalBounds {
            x: fallback.x,
            y: fallback.y,
            width: 1,
            height: 1,
        }
    }
}

fn hover_physical_size(window: &WebviewWindow, target_scale_factor: f64) -> PhysicalSize<u32> {
    let current_scale_factor = window.scale_factor().unwrap_or(target_scale_factor);
    window
        .outer_size()
        .map(|size| {
            let logical = size.to_logical::<f64>(current_scale_factor);
            PhysicalSize::new(
                (logical.width * target_scale_factor).round().max(1.0) as u32,
                (logical.height * target_scale_factor).round().max(1.0) as u32,
            )
        })
        .unwrap_or_else(|_| {
            PhysicalSize::new(
                (DEFAULT_HOVER_WIDTH as f64 * target_scale_factor).round() as u32,
                (DEFAULT_HOVER_HEIGHT as f64 * target_scale_factor).round() as u32,
            )
        })
}

fn window_bounds(window: &WebviewWindow) -> Option<PhysicalBounds> {
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(PhysicalBounds {
        x: position.x,
        y: position.y,
        width: size.width as i32,
        height: size.height as i32,
    })
}

fn reveal_webview_window(window: &WebviewWindow, activate_virtual_desktop: bool) {
    let _ = window.unminimize();
    let _ = window.show();
    if activate_virtual_desktop {
        let _ = window.set_focus();
        activate_for_virtual_desktop(window);
    }
}

#[cfg(target_os = "windows")]
fn activate_for_virtual_desktop(window: &WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        let raw_hwnd = hwnd.0 as _;
        unsafe {
            if window.is_minimized().unwrap_or(false) {
                ShowWindow(raw_hwnd, SW_RESTORE);
            }
            SetForegroundWindow(raw_hwnd);
        }
    }

    let _ = window.set_focus();
}

#[cfg(not(target_os = "windows"))]
fn activate_for_virtual_desktop(_window: &WebviewWindow) {}

fn hover_position_near(
    anchor: PhysicalBounds,
    hover_size: PhysicalSize<u32>,
    gap: i32,
    monitor: Option<tauri::Monitor>,
) -> PhysicalPosition<i32> {
    let hover_width = hover_size.width as i32;
    let hover_height = hover_size.height as i32;
    let center_x = anchor.x + anchor.width / 2;
    let center_y = anchor.y + anchor.height / 2;
    let candidates = [
        PhysicalPosition::new(center_x - hover_width / 2, anchor.y - hover_height - gap),
        PhysicalPosition::new(center_x - hover_width / 2, anchor.y + anchor.height + gap),
        PhysicalPosition::new(anchor.x - hover_width - gap, center_y - hover_height / 2),
        PhysicalPosition::new(anchor.x + anchor.width + gap, center_y - hover_height / 2),
    ];

    let Some(monitor) = monitor else {
        return candidates[0];
    };
    let area = monitor.work_area();
    let area_left = area.position.x;
    let area_top = area.position.y;
    let area_right = area_left + area.size.width as i32;
    let area_bottom = area_top + area.size.height as i32;

    if let Some(position) = candidates.into_iter().find(|position| {
        position.x >= area_left
            && position.y >= area_top
            && position.x + hover_width <= area_right
            && position.y + hover_height <= area_bottom
    }) {
        return position;
    }

    let fallback = candidates[0];
    PhysicalPosition::new(
        fallback
            .x
            .clamp(area_left, (area_right - hover_width).max(area_left)),
        fallback
            .y
            .clamp(area_top, (area_bottom - hover_height).max(area_top)),
    )
}

pub fn hide_hover_window(app: &AppHandle) -> bool {
    debug_log::line("hide_hover_window: begin");
    let result = app
        .get_webview_window("hover")
        .map(|window| window.hide())
        .transpose();
    debug_log::line(format!("hide_hover_window: end result={result:?}"));
    matches!(result, Ok(Some(())) | Ok(None))
}

pub fn hide_window(app: &AppHandle, label: &str) {
    if label == "hover" {
        tray::mark_hover_hidden();
    }
    debug_log::line(format!("hide_window: {label}: begin"));
    if let Some(window) = app.get_webview_window(label) {
        let result = window.hide();
        debug_log::line(format!("hide_window: {label}: end result={result:?}"));
    } else {
        debug_log::line(format!("hide_window: {label}: window not found"));
    }
}

pub fn open_main_overview(app: &AppHandle) {
    debug_log::line("open_main_overview");
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
    debug_log::line("open_main_update");
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

        let bounds = tray_hover_bounds(rect, fallback, 1.5);

        assert_eq!(
            bounds,
            PhysicalBounds {
                x: 150,
                y: 300,
                width: 36,
                height: 36
            }
        );
    }

    #[test]
    fn tray_hover_anchor_falls_back_when_rect_is_empty() {
        let rect = Rect {
            position: Position::Logical(LogicalPosition::new(100.0, 200.0)),
            size: Size::Logical(LogicalSize::new(0.0, 0.0)),
        };
        let fallback = PhysicalPosition::new(500.0, 600.0);

        let bounds = tray_hover_bounds(rect, fallback, 1.5);

        assert_eq!(
            bounds,
            PhysicalBounds {
                x: 500,
                y: 600,
                width: 1,
                height: 1
            }
        );
    }

    #[test]
    fn main_window_state_enforces_minimum_size() {
        let state = normalize_main_window_state(MainWindowState {
            width: 320,
            height: 240,
            x: 10,
            y: 20,
            maximized: false,
            fullscreen: false,
        });

        assert_eq!(state.width, MAIN_WINDOW_MIN_WIDTH);
        assert_eq!(state.height, MAIN_WINDOW_MIN_HEIGHT);
        assert_eq!(state.x, 10);
        assert_eq!(state.y, 20);
    }
}
