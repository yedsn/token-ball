use std::sync::Arc;

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

use crate::{
    app_state::AppState,
    commands,
    quota::{ConnectionStatus, DisplaySettings, QuotaSummary},
    storage::repository,
    windows,
};

const TRAY_ID: &str = "token-ball";

pub fn setup_tray(
    app: &tauri::App,
    initial_summary: &QuotaSummary,
    initial_settings: &DisplaySettings,
) -> tauri::Result<()> {
    let show_orb = MenuItem::with_id(app, "show_orb", "显示余量球", true, None::<&str>)?;
    let hide_orb = MenuItem::with_id(app, "hide_orb", "隐藏余量球", true, None::<&str>)?;
    let open_main = MenuItem::with_id(app, "open_main", "打开管理", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_orb, &hide_orb, &open_main, &refresh, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(tray_icon_for_summary(initial_summary, initial_settings))
        .tooltip(tray_tooltip(initial_summary, initial_settings))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_orb" => set_orb_visible(app, true),
            "hide_orb" => {
                windows::hide_window(app, "hover");
                set_orb_visible(app, false);
            }
            "open_main" => windows::open_main_overview(app),
            "refresh" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<Arc<AppState>>();
                    let _ = commands::quota::refresh_all_internal(&app_handle, &state).await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                windows::open_main_overview(tray.app_handle());
            }
            TrayIconEvent::Enter { .. } => {
                let app = tray.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    update_tray_from_storage(&app).await;
                });
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
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
        let _ = tray.set_icon(Some(tray_icon_for_summary(summary, settings)));
        let _ = tray.set_tooltip(Some(tray_tooltip(summary, settings)));
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
            lines.push(format!("- {}：{}", account.display_name, remaining));
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
