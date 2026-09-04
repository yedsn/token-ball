mod app_icon_rgba;
mod app_state;
mod commands;
mod debug_log;
mod error;
mod events;
mod providers;
mod quota;
mod scheduler;
mod storage;
mod tray;
mod windows;

use std::sync::Arc;

use app_state::AppState;
use tauri::{Manager, WindowEvent};

pub fn run() {
    debug_log::init();
    debug_log::line("app run start");

    tracing_subscriber::fmt()
        .with_env_filter("token_ball=info,warn")
        .without_time()
        .init();

    let (app_state, initial_summary, initial_settings) = tauri::async_runtime::block_on(async {
        let db = storage::init_database()
            .await
            .map_err(|error| error.to_string())?;
        storage::repository::ensure_default_settings(&db)
            .await
            .map_err(|error| error.to_string())?;
        storage::repository::ensure_default_plugins(&db)
            .await
            .map_err(|error| error.to_string())?;
        let initial_summary = storage::repository::load_summary(&db, false)
            .await
            .unwrap_or_default();
        let initial_settings = storage::repository::load_display_settings(&db)
            .await
            .unwrap_or_default();
        let main_window_state =
            storage::repository::get_setting(&db, windows::MAIN_WINDOW_STATE_KEY)
                .await
                .ok()
                .flatten()
                .and_then(|value| windows::parse_main_window_state(&value));
        Ok::<_, String>((
            Arc::new(AppState::new(db, main_window_state)),
            initial_summary,
            initial_settings,
        ))
    })
    .expect("初始化 TokenBall 状态失败");

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            crate::debug_log::line("single instance: show main");
            crate::windows::show_window(app, "main");
        }))
        .setup(move |app| {
            crate::debug_log::line("setup start");
            tray::setup_tray(app, &initial_summary, &initial_settings)?;
            let _ = commands::app_icon_set_style(
                app.handle().clone(),
                initial_settings.app_icon_style.clone(),
                Some(initial_settings.custom_app_icon_data_url.clone()),
            );
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tray::apply_orb_visibility(&app_handle).await;
            });
            scheduler::start_quota_scheduler(app.handle().clone());
            // 启动后静默检查更新，前端通过 updater://status 事件接收结果。
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                commands::updater::run_startup_check(app_handle).await;
            });

            if let Some(main_window) = app.get_webview_window("main") {
                crate::windows::apply_cached_main_window_state(app.handle(), &main_window);
                let app_handle = app.handle().clone();
                let main_window_for_events = main_window.clone();
                main_window.on_window_event(move |event| {
                    match event {
                        WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            crate::windows::handle_main_close(&app_handle);
                        }
                        WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                            crate::windows::save_main_window_state_delayed(
                                &app_handle,
                                &main_window_for_events,
                            );
                        }
                        WindowEvent::Focused(false) => {
                            crate::windows::save_main_window_state_delayed(
                                &app_handle,
                                &main_window_for_events,
                            );
                        }
                        _ => {}
                    }
                });
            }
            crate::debug_log::line("setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection_list,
            commands::connection_save,
            commands::connection_export_config_to_file,
            commands::connection_import_config,
            commands::connection_read_config_backup,
            commands::connection_import_config_from_file,
            commands::connection_delete,
            commands::connection_test,
            commands::connection_set_enabled,
            commands::plugin_list,
            commands::plugin_set_enabled,
            commands::plugin_add,
            commands::plugin_delete,
            commands::app_icon_set_style,
            commands::quota_get_latest,
            commands::quota_refresh_all,
            commands::quota_refresh_connection,
            commands::settings_get_display,
            commands::settings_save_display,
            commands::updater_check,
            commands::updater_download_and_install,
            commands::updater_restart,
            commands::window_show,
            commands::window_hide,
            commands::window_hide_hover_after_keyboard_close,
            commands::window_open_main_overview,
            commands::window_minimize_main,
            commands::window_toggle_main_maximize,
            commands::window_close_main,
            commands::open_external_url,
            commands::orb_get_visible
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenBall");
}
