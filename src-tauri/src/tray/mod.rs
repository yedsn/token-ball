use std::{
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Rect,
};

use chrono::{DateTime, Utc};

use crate::{
    app_state::AppState,
    commands, debug_log,
    quota::{ConnectionStatus, DisplaySettings, QuotaSummary},
    storage::repository,
    windows,
};

const TRAY_ID: &str = "token-ball";
const HOVER_MOVE_THROTTLE: Duration = Duration::from_millis(120);
const HOVER_SHOW_DELAY: Duration = Duration::from_millis(300);
const HOVER_OUTSIDE_GRACE: Duration = Duration::from_secs(3);
const HOVER_MONITOR_INTERVAL: Duration = Duration::from_millis(25);
const KEYBOARD_CLOSE_SUPPRESS_HOVER: Duration = Duration::from_millis(300);
const TRAY_MENU_SUPPRESS_HOVER: Duration = Duration::from_millis(1200);

static TRAY_HOVER_STATE: OnceLock<Arc<Mutex<TrayHoverState>>> = OnceLock::new();

#[derive(Debug)]
struct TrayHoverState {
    generation: u64,
    pending_show: bool,
    shown: bool,
    last_move: Instant,
    last_rect: Rect,
    last_position: tauri::PhysicalPosition<f64>,
    suppress_until: Instant,
}

pub fn setup_tray(
    app: &tauri::App,
    initial_summary: &QuotaSummary,
    initial_settings: &DisplaySettings,
) -> tauri::Result<()> {
    debug_log::line("setup_tray");
    let hover_state = Arc::new(Mutex::new(TrayHoverState {
        generation: 0,
        pending_show: false,
        shown: false,
        last_move: Instant::now() - HOVER_MOVE_THROTTLE,
        last_rect: Rect::default(),
        last_position: tauri::PhysicalPosition::new(0.0, 0.0),
        suppress_until: Instant::now(),
    }));
    let _ = TRAY_HOVER_STATE.set(Arc::clone(&hover_state));
    let show_orb = MenuItem::with_id(app, "show_orb", "显示额度", true, None::<&str>)?;
    let hide_orb = MenuItem::with_id(app, "hide_orb", "隐藏额度", true, None::<&str>)?;
    let open_main = MenuItem::with_id(app, "open_main", "打开管理", true, None::<&str>)?;
    let reset_main = MenuItem::with_id(app, "reset_main", "重置窗口", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
    let check_update = MenuItem::with_id(app, "check_update", "检查更新", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_orb,
            &hide_orb,
            &open_main,
            &reset_main,
            &refresh,
            &check_update,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(tray_icon_for_summary(initial_summary, initial_settings))
        .tooltip(tray_tooltip(initial_summary, initial_settings))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_orb" => {
                debug_log::line("tray menu: show_orb");
                windows::hide_window(app, "hover");
                set_orb_visible(app, true)
            }
            "hide_orb" => {
                debug_log::line("tray menu: hide_orb");
                windows::hide_window(app, "hover");
                set_orb_visible(app, false);
            }
            "open_main" => {
                debug_log::line("tray menu: open_main");
                windows::open_main_overview(app)
            }
            "reset_main" => {
                debug_log::line("tray menu: reset_main");
                windows::reset_main_window(app)
            }
            "check_update" => {
                debug_log::line("tray menu: check_update");
                windows::open_main_update(app)
            }
            "refresh" => {
                debug_log::line("tray menu: refresh");
                windows::hide_window(app, "hover");
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<Arc<AppState>>();
                    let _ = commands::quota::refresh_all_internal(&app_handle, &state).await;
                });
            }
            "quit" => {
                debug_log::line("tray menu: quit");
                windows::hide_window(app, "hover");
                let value = app
                    .get_webview_window("main")
                    .and_then(|window| windows::main_window_state_json(&window));
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(value) = value {
                        let state = app_handle.state::<Arc<AppState>>();
                        let _ =
                            repository::set_setting(&state.db, "window.main.state", &value).await;
                    }
                    app_handle.exit(0);
                });
            }
            _ => {}
        })
        .on_tray_icon_event({
            let hover_state = Arc::clone(&hover_state);
            move |tray, event| match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    debug_log::line("tray icon: left click up");
                    cancel_tray_hover(&hover_state);
                    windows::hide_window(tray.app_handle(), "hover");
                    windows::open_main_overview(tray.app_handle());
                }
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Down,
                    ..
                } => {
                    debug_log::line("tray icon: right click");
                    suppress_tray_hover(&hover_state, TRAY_MENU_SUPPRESS_HOVER);
                    windows::hide_window(tray.app_handle(), "hover");
                }
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => {
                    debug_log::line("tray icon: left double click");
                    cancel_tray_hover(&hover_state);
                    windows::hide_window(tray.app_handle(), "hover");
                    windows::open_main_overview(tray.app_handle());
                }
                TrayIconEvent::Enter { position, rect, .. }
                | TrayIconEvent::Move { position, rect, .. } => {
                    let is_move = matches!(event, TrayIconEvent::Move { .. });
                    let mut should_schedule = false;
                    let generation = {
                        let Ok(mut state) = hover_state.lock() else {
                            return;
                        };
                        if is_move && state.last_move.elapsed() < HOVER_MOVE_THROTTLE {
                            return;
                        }
                        if Instant::now() < state.suppress_until {
                            return;
                        }
                        if is_move {
                            state.last_move = Instant::now();
                        }
                        state.last_rect = rect;
                        state.last_position = position;

                        if !state.shown && !state.pending_show {
                            state.generation = state.generation.wrapping_add(1);
                            state.pending_show = true;
                            should_schedule = true;
                        }
                        state.generation
                    };

                    let app = tray.app_handle().clone();
                    if should_schedule {
                        let hover_state = Arc::clone(&hover_state);
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(HOVER_SHOW_DELAY).await;
                            let ui_app = app.clone();
                            let ui_state = Arc::clone(&hover_state);
                            if let Err(error) = app.run_on_main_thread(move || {
                                let app = ui_app;
                                let hover_state = ui_state;
                                let Some((tray_bounds, hover_bounds)) = commit_pending_hover(
                                    &hover_state,
                                    generation,
                                    |rect, position| {
                                        let bounds =
                                            windows::tray_bounds_for_event(&app, rect, position);
                                        cursor_position(&app)
                                            .is_some_and(|cursor| bounds.contains(cursor))
                                    },
                                    |rect, position| {
                                        debug_log::line("tray icon: show hover after delay");
                                        windows::show_hover_near_tray(&app, rect, position)
                                    },
                                ) else {
                                    return;
                                };
                                monitor_visible_hover(
                                    app.clone(),
                                    Arc::clone(&hover_state),
                                    generation,
                                    tray_bounds,
                                    hover_bounds,
                                );
                            }) {
                                clear_pending_hover(&hover_state, generation);
                                debug_log::line(format!("tray hover dispatch failed: {error}"));
                            }
                        });
                    }
                }
                TrayIconEvent::Leave { .. } => {
                    debug_log::line("tray icon: leave");
                    cancel_pending_tray_hover(&hover_state);
                }
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

pub fn hide_hover_after_keyboard_close(app: &tauri::AppHandle) {
    debug_log::line("hide_hover_after_keyboard_close");
    hide_tray_hover(app, KEYBOARD_CLOSE_SUPPRESS_HOVER);
}

pub fn hide_tray_hover(app: &tauri::AppHandle, suppression: Duration) {
    let ui_app = app.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        if let Some(state) = TRAY_HOVER_STATE.get() {
            suppress_tray_hover(state, suppression);
        }
        windows::hide_hover_window(&ui_app);
    }) {
        debug_log::line(format!("tray hover hide dispatch failed: {error}"));
    }
}

fn clear_pending_hover(hover_state: &Mutex<TrayHoverState>, generation: u64) {
    if let Ok(mut state) = hover_state.lock() {
        if state.generation == generation {
            state.pending_show = false;
        }
    }
}

// UI-thread only: never hold this lock across queries or native window calls.
// Tauri getters can synchronously wait for the UI event loop.
fn commit_pending_hover<T>(
    hover_state: &Mutex<TrayHoverState>,
    generation: u64,
    query: impl FnOnce(Rect, tauri::PhysicalPosition<f64>) -> bool,
    show: impl FnOnce(Rect, tauri::PhysicalPosition<f64>) -> Option<T>,
) -> Option<T> {
    let (rect, position) = {
        let state = hover_state.lock().ok()?;
        if state.generation != generation || !state.pending_show {
            return None;
        }
        (state.last_rect, state.last_position)
    };
    if !query(rect, position) {
        clear_pending_hover(hover_state, generation);
        return None;
    }
    {
        let mut state = hover_state.lock().ok()?;
        if state.generation != generation || !state.pending_show {
            return None;
        }
        if Instant::now() < state.suppress_until {
            state.pending_show = false;
            return None;
        }
    }
    let result = show(rect, position);
    let mut state = hover_state.lock().ok()?;
    // Reentrant actions own the surface now; never hide a newer window.
    if state.generation != generation {
        return None;
    }
    state.pending_show = false;
    state.shown = result.is_some();
    result
}

fn cancel_pending_tray_hover(hover_state: &Arc<Mutex<TrayHoverState>>) {
    let Ok(mut state) = hover_state.lock() else {
        return;
    };
    if state.pending_show {
        state.generation = state.generation.wrapping_add(1);
        state.pending_show = false;
    }
}

fn cancel_tray_hover(hover_state: &Arc<Mutex<TrayHoverState>>) {
    let Ok(mut state) = hover_state.lock() else {
        return;
    };
    state.generation = state.generation.wrapping_add(1);
    state.pending_show = false;
    state.shown = false;
}

fn suppress_tray_hover(hover_state: &Arc<Mutex<TrayHoverState>>, duration: Duration) {
    let Ok(mut state) = hover_state.lock() else {
        return;
    };
    state.generation = state.generation.wrapping_add(1);
    state.pending_show = false;
    state.shown = false;
    state.suppress_until = state.suppress_until.max(Instant::now() + duration);
}

fn monitor_visible_hover(
    app: tauri::AppHandle,
    hover_state: Arc<Mutex<TrayHoverState>>,
    generation: u64,
    tray_bounds: windows::PhysicalBounds,
    hover_bounds: windows::PhysicalBounds,
) {
    tauri::async_runtime::spawn(async move {
        let mut outside_since = None;
        let mut previous_buttons = mouse_buttons_pressed();

        loop {
            let active = hover_state
                .lock()
                .map(|state| state.shown && state.generation == generation)
                .unwrap_or(false);
            if !active {
                return;
            }

            let cursor = cursor_position(&app);
            let inside_interactive_region = cursor
                .map(|position| tray_bounds.contains(position) || hover_bounds.contains(position))
                .unwrap_or(true);
            let current_buttons = mouse_buttons_pressed();
            let new_outside_click = !inside_interactive_region
                && has_new_button_press(current_buttons, previous_buttons);
            previous_buttons = current_buttons;

            let should_hide = if inside_interactive_region {
                outside_since = None;
                false
            } else if new_outside_click {
                true
            } else {
                let started = outside_since.get_or_insert_with(Instant::now);
                started.elapsed() >= HOVER_OUTSIDE_GRACE
            };

            if should_hide {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                let ui_app = app.clone();
                let ui_state = Arc::clone(&hover_state);
                if let Err(error) = app.run_on_main_thread(move || {
                    let hidden = hide_current_hover(&ui_state, generation, || {
                        windows::hide_hover_window(&ui_app)
                    });
                    let _ = sender.send(hidden);
                }) {
                    debug_log::line(format!("tray hover monitor dispatch failed: {error}"));
                    return;
                }
                if receiver.await.unwrap_or(true) {
                    return;
                }
                tokio::time::sleep(HOVER_MONITOR_INTERVAL).await;
            }

            let sleep_for = outside_since
                .map(|started| {
                    HOVER_MONITOR_INTERVAL
                        .min(HOVER_OUTSIDE_GRACE.saturating_sub(started.elapsed()))
                })
                .unwrap_or(HOVER_MONITOR_INTERVAL);
            tokio::time::sleep(sleep_for.max(Duration::from_millis(1))).await;
        }
    });
}

// UI-thread only: check ownership immediately before hiding, without the lock.
fn hide_current_hover(
    hover_state: &Mutex<TrayHoverState>,
    generation: u64,
    hide: impl FnOnce() -> bool,
) -> bool {
    let active = hover_state
        .lock()
        .map(|state| state.generation == generation && state.shown)
        .unwrap_or(false);
    if !active {
        return true;
    }
    if !hide() {
        return false;
    }
    if let Ok(mut state) = hover_state.lock() {
        if state.generation == generation {
            state.generation = state.generation.wrapping_add(1);
            state.shown = false;
            state.pending_show = false;
        }
    }
    true
}

fn has_new_button_press(current: [bool; 3], previous: [bool; 3]) -> bool {
    current
        .into_iter()
        .zip(previous)
        .any(|(current, previous)| current && !previous)
}

#[cfg(windows)]
fn cursor_position(_app: &tauri::AppHandle) -> Option<tauri::PhysicalPosition<i32>> {
    let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
    let ok = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point) };
    if ok == 0 {
        None
    } else {
        Some(tauri::PhysicalPosition::new(point.x, point.y))
    }
}

#[cfg(windows)]
fn mouse_buttons_pressed() -> [bool; 3] {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
    };

    [VK_LBUTTON, VK_MBUTTON, VK_RBUTTON]
        .map(|button| unsafe { GetAsyncKeyState(button as i32) } < 0)
}

#[cfg(not(windows))]
fn cursor_position(app: &tauri::AppHandle) -> Option<tauri::PhysicalPosition<i32>> {
    app.cursor_position().ok().map(|position| position.cast())
}

#[cfg(target_os = "macos")]
fn mouse_buttons_pressed() -> [bool; 3] {
    // AppKit exposes global button state without requiring the panel to take focus.
    let pressed = objc2_app_kit::NSEvent::pressedMouseButtons();
    [1, 2, 4].map(|mask| pressed & mask != 0)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn mouse_buttons_pressed() -> [bool; 3] {
    [false; 3]
}

pub async fn apply_orb_visibility(app: &tauri::AppHandle) {
    let state = app.state::<Arc<AppState>>();
    let visible = repository::get_bool_setting(&state.db, "orb.visible", true)
        .await
        .unwrap_or(true);
    if visible {
        windows::show_window(app, "orb");
    } else {
        windows::hide_window(app, "hover");
        windows::hide_window(app, "orb");
    }
}

fn set_orb_visible(app: &tauri::AppHandle, visible: bool) {
    debug_log::line(format!("set_orb_visible: {visible}"));
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<Arc<AppState>>();
        let _ = repository::set_bool_setting(&state.db, "orb.visible", visible).await;
    });
    if visible {
        windows::show_window(app, "orb");
    } else {
        windows::hide_window(app, "orb");
    }
}

pub async fn update_tray_from_storage(app: &tauri::AppHandle) {
    let state = app.state::<Arc<AppState>>();
    if let (Ok(summary), Ok(settings)) = (
        repository::load_summary(&state.db, false).await,
        repository::load_display_settings(&state.db).await,
    ) {
        update_tray_with_settings(app, &summary, &settings);
    }
}

pub async fn update_tray(app: &tauri::AppHandle, summary: &QuotaSummary) {
    let state = app.state::<Arc<AppState>>();
    let settings = repository::load_display_settings(&state.db)
        .await
        .unwrap_or_default();
    update_tray_with_settings(app, summary, &settings);
}

fn update_tray_with_settings(
    app: &tauri::AppHandle,
    summary: &QuotaSummary,
    settings: &DisplaySettings,
) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        debug_log::line("tray update: set_icon begin");
        let icon_result = tray.set_icon(Some(tray_icon_for_summary(summary, settings)));
        debug_log::line(format!("tray update: set_icon end result={icon_result:?}"));

        debug_log::line("tray update: set_tooltip begin");
        let tooltip_result = tray.set_tooltip(Some(tray_tooltip(summary, settings)));
        debug_log::line(format!(
            "tray update: set_tooltip end result={tooltip_result:?}"
        ));
    }
}

fn summary_percent(summary: &QuotaSummary) -> Option<f64> {
    let values: Vec<f64> = summary
        .accounts
        .iter()
        .filter_map(|account| {
            account
                .critical_window_id
                .as_ref()
                .and_then(|id| account.windows.iter().find(|window| &window.id == id))
                .and_then(|window| window.remaining_percent)
        })
        .collect();
    if values.is_empty() {
        summary.lowest_remaining_percent
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn tray_tooltip(summary: &QuotaSummary, settings: &DisplaySettings) -> String {
    let percent = summary_percent(summary)
        .map(|value| format!("{:.0}%", value))
        .unwrap_or_else(|| "未知".to_string());
    let mut lines = Vec::new();

    if settings.show_total_remaining {
        lines.push(format!("总额度：{percent}"));
    }
    if settings.show_available_accounts {
        lines.push(format!(
            "可用账号：{} / {}",
            summary.available_accounts, summary.total_accounts
        ));
    }
    if settings.show_connection_status {
        lines.push(format!("连接状态：{}", status_label(&summary.status)));
    }

    let account_remaining: Vec<_> = summary
        .accounts
        .iter()
        .map(|account| {
            let account_percent = account
                .critical_window_id
                .as_ref()
                .and_then(|id| account.windows.iter().find(|window| &window.id == id))
                .and_then(|window| window.remaining_percent);
            (account, account_percent)
        })
        .collect();

    if settings.show_accounts_in_tooltip {
        if account_remaining.is_empty() {
            lines.push("账号剩余额度：暂无".to_string());
        } else {
            lines.push("账号剩余额度：".to_string());
        }

        for (account, account_percent) in account_remaining {
            let remaining = account_percent
                .map(|value| format!("{:.0}%", value))
                .unwrap_or_else(|| "未知".to_string());
            let mut line = format!("- {}：{}", account.display_name, remaining);
            if let Some(expiry) = account.subscription_until.map(format_date) {
                line.push_str(&format!(" | 到期 {}", expiry));
            }
            lines.push(line);
        }
    }

    for item in settings.custom_items.iter().filter(|item| item.enabled) {
        if !item.label.trim().is_empty() || !item.value.trim().is_empty() {
            lines.push(format!("{}：{}", item.label.trim(), item.value.trim()));
        }
    }

    if lines.is_empty() {
        lines.push("TokenBall：暂无显示内容".to_string());
    }
    lines.join("\n")
}

fn format_date(value: DateTime<Utc>) -> String {
    value.format("%m-%d %H:%M").to_string()
}

fn status_label(status: &ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Healthy => "healthy",
        ConnectionStatus::Degraded => "degraded",
        ConnectionStatus::Failed => "failed",
        ConnectionStatus::Unknown => "unknown",
    }
}

fn tray_icon_for_summary(summary: &QuotaSummary, settings: &DisplaySettings) -> Image<'static> {
    let percent = summary_percent(summary).map(|value| value.clamp(0.0, 100.0));
    let color = if summary.stale || summary.status == ConnectionStatus::Degraded {
        (127, 142, 163)
    } else if let Some(value) = percent {
        if value < 30.0 {
            (255, 90, 84)
        } else if value < 60.0 {
            (233, 185, 73)
        } else {
            (25, 195, 125)
        }
    } else {
        (72, 166, 255)
    };
    if settings.tray_icon_style == "minimal" {
        draw_minimal_icon(color)
    } else {
        draw_orb_icon(percent.unwrap_or(0.0), color)
    }
}

pub fn quota_orb_icon(percent: f64, status: ConnectionStatus, stale: bool) -> Image<'static> {
    let color = if stale || status == ConnectionStatus::Degraded {
        (127, 142, 163)
    } else if percent < 30.0 {
        (255, 90, 84)
    } else if percent < 60.0 {
        (233, 185, 73)
    } else {
        (25, 195, 125)
    };
    draw_orb_icon(percent.clamp(0.0, 100.0), color)
}

fn draw_minimal_icon(color: (u8, u8, u8)) -> Image<'static> {
    let size = 32usize;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let index = (y * size + x) * 4;
            let border = x < 5 || x >= size - 5 || y < 5 || y >= size - 5;
            let color = if border {
                blend(color, (255, 255, 255), 0.26)
            } else {
                color
            };
            rgba[index] = color.0;
            rgba[index + 1] = color.1;
            rgba[index + 2] = color.2;
            rgba[index + 3] = 255;
        }
    }
    Image::new_owned(rgba, size as u32, size as u32)
}

fn draw_orb_icon(percent: f64, liquid: (u8, u8, u8)) -> Image<'static> {
    let size = 32usize;
    let center = 15.5f64;
    let radius = 14.2f64;
    let liquid_top = size as f64 - 3.0 - (percent / 100.0 * 26.0);
    let mut rgba = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > radius {
                continue;
            }

            let edge = ((radius - distance) / 1.4).clamp(0.0, 1.0);
            let is_liquid = y as f64 >= liquid_top + (dx / 3.8).sin() * 1.1;
            let mut color = if is_liquid { liquid } else { (12, 20, 18) };

            if distance > radius - 1.8 {
                color = blend(color, (232, 240, 238), 0.42);
            }
            if x < 12 && y < 12 && distance < radius - 3.0 {
                color = blend(color, (255, 255, 255), 0.24);
            }
            if is_liquid && y as f64 <= liquid_top + 2.4 {
                color = blend(color, (255, 255, 255), 0.28);
            }

            let index = (y * size + x) * 4;
            rgba[index] = color.0;
            rgba[index + 1] = color.1;
            rgba[index + 2] = color.2;
            rgba[index + 3] = (255.0 * edge.max(0.78)) as u8;
        }
    }

    Image::new_owned(rgba, size as u32, size as u32)
}

fn blend(base: (u8, u8, u8), top: (u8, u8, u8), amount: f64) -> (u8, u8, u8) {
    let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * amount) as u8;
    (mix(base.0, top.0), mix(base.1, top.1), mix(base.2, top.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_state() -> Arc<Mutex<TrayHoverState>> {
        Arc::new(Mutex::new(TrayHoverState {
            generation: 1,
            pending_show: true,
            shown: false,
            last_move: Instant::now(),
            last_rect: Rect::default(),
            last_position: tauri::PhysicalPosition::new(0.0, 0.0),
            suppress_until: Instant::now(),
        }))
    }

    #[test]
    fn blocked_query_does_not_block_cancellation() {
        use std::sync::mpsc;
        let state = pending_state();
        let worker_state = Arc::clone(&state);
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let result = commit_pending_hover(
                &worker_state,
                1,
                |_, _| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                    true
                },
                |_, _| -> Option<()> { panic!("cancelled request displayed") },
            );
            done_tx.send(result).unwrap();
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let acquired = state.try_lock().is_ok();
        if acquired {
            cancel_pending_tray_hover(&state);
        }
        release_tx.send(()).unwrap();
        assert!(acquired, "query retained the hover lock");
        assert!(done_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_none());
        worker.join().unwrap();
    }

    #[test]
    fn all_cancel_actions_invalidate_prepared_hover() {
        for action in 0..4 {
            let state = pending_state();
            let result = commit_pending_hover(
                &state,
                1,
                |_, _| {
                    match action {
                        0 => cancel_pending_tray_hover(&state),
                        1 => cancel_tray_hover(&state),
                        2 => suppress_tray_hover(&state, TRAY_MENU_SUPPRESS_HOVER),
                        _ => suppress_tray_hover(&state, KEYBOARD_CLOSE_SUPPRESS_HOVER),
                    }
                    true
                },
                |_, _| -> Option<()> { panic!("cancelled request displayed") },
            );
            assert!(result.is_none());
        }
    }

    #[test]
    fn generic_hide_preserves_menu_suppression() {
        let state = pending_state();
        suppress_tray_hover(&state, TRAY_MENU_SUPPRESS_HOVER);
        let deadline = state.lock().unwrap().suppress_until;
        suppress_tray_hover(&state, Duration::ZERO);
        assert_eq!(state.lock().unwrap().suppress_until, deadline);
        state.lock().unwrap().pending_show = true;
        let generation = state.lock().unwrap().generation;
        assert!(commit_pending_hover(
            &state,
            generation,
            |_, _| true,
            |_, _| -> Option<()> { panic!("displayed during suppression") }
        )
        .is_none());
        state.lock().unwrap().suppress_until = Instant::now();
        state.lock().unwrap().pending_show = true;
        assert_eq!(
            commit_pending_hover(&state, generation, |_, _| true, |_, _| Some(())),
            Some(())
        );
    }

    #[test]
    fn failures_allow_subsequent_hover() {
        for failure in 0..3 {
            let state = pending_state();
            if failure == 0 {
                clear_pending_hover(&state, 1); // dispatch failed
            } else {
                let result =
                    commit_pending_hover(&state, 1, |_, _| failure != 1, |_, _| None::<()>);
                assert!(result.is_none());
            }
            {
                let mut current = state.lock().unwrap();
                assert!(!current.pending_show && !current.shown);
                current.generation += 1;
                current.pending_show = true;
            }
            assert_eq!(
                commit_pending_hover(
                    &state,
                    2,
                    |_, _| true,
                    |_, _| {
                        assert!(state.try_lock().is_ok(), "show retained the lock");
                        Some(())
                    }
                ),
                Some(())
            );
        }
    }

    #[test]
    fn old_request_and_monitor_cannot_change_new_panel() {
        let state = pending_state();
        let result = commit_pending_hover(
            &state,
            1,
            |_, _| {
                cancel_tray_hover(&state);
                state.lock().unwrap().shown = true;
                true
            },
            |_, _| -> Option<()> { panic!("old request displayed") },
        );
        assert!(result.is_none());
        clear_pending_hover(&state, 1);
        assert!(hide_current_hover(&state, 1, || panic!(
            "old monitor hid new panel"
        )));
        assert!(state.lock().unwrap().shown);
    }

    #[test]
    fn reentrant_show_preserves_new_owner() {
        let state = pending_state();
        assert!(commit_pending_hover(
            &state,
            1,
            |_, _| true,
            |_, _| {
                cancel_tray_hover(&state);
                state.lock().unwrap().pending_show = true;
                Some(())
            }
        )
        .is_none());
        assert!(state.lock().unwrap().pending_show);
    }

    #[test]
    fn hide_failure_retries_without_holding_lock() {
        let state = pending_state();
        {
            let mut current = state.lock().unwrap();
            current.pending_show = false;
            current.shown = true;
        }
        assert!(!hide_current_hover(&state, 1, || {
            assert!(state.try_lock().is_ok());
            false
        }));
        assert!(state.lock().unwrap().shown);
        assert!(hide_current_hover(&state, 1, || true));
        assert!(!state.lock().unwrap().shown);
    }

    #[test]
    fn physical_bounds_use_inclusive_start_and_exclusive_end() {
        let bounds = windows::PhysicalBounds {
            x: 100,
            y: 200,
            width: 24,
            height: 24,
        };
        assert!(bounds.contains(tauri::PhysicalPosition::new(100, 200)));
        assert!(bounds.contains(tauri::PhysicalPosition::new(123, 223)));
        assert!(!bounds.contains(tauri::PhysicalPosition::new(99, 210)));
        assert!(!bounds.contains(tauri::PhysicalPosition::new(110, 224)));
    }

    #[test]
    fn held_mouse_button_is_not_treated_as_a_new_press() {
        assert!(!has_new_button_press(
            [true, false, false],
            [true, false, false]
        ));
        assert!(has_new_button_press(
            [true, false, false],
            [false, false, false]
        ));
    }
}
