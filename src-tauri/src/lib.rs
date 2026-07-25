mod app_state;
mod commands;
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

pub fn run() {
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
        Ok::<_, String>((
            Arc::new(AppState::new(db)),
            initial_summary,
            initial_settings,
        ))
    })
    .expect("初始化 TokenBall 状态失败");

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            crate::windows::show_window(app, "main");
        }))
        .setup(move |app| {
            tray::setup_tray(app, &initial_summary, &initial_settings)?;
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tray::apply_orb_visibility(&app_handle).await;
            });
            scheduler::start_quota_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection_list,
            commands::connection_save,
            commands::connection_delete,
            commands::connection_test,
            commands::connection_set_enabled,
            commands::plugin_list,
            commands::plugin_set_enabled,
            commands::plugin_add,
            commands::plugin_delete,
            commands::quota_get_latest,
            commands::quota_refresh_all,
            commands::settings_get_display,
            commands::settings_save_display,
            commands::window_show,
            commands::window_hide,
            commands::orb_get_visible
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenBall");
}
