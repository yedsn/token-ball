mod app_state;
mod commands;
mod error;
mod events;
mod providers;
mod quota;
mod scheduler;
mod storage;
mod windows;

use std::sync::Arc;

use app_state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("token_ball=info,warn")
        .without_time()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            crate::windows::show_window(app, "main");
        }))
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let db = storage::init_database()
                    .await
                    .map_err(|error| error.to_string())?;
                storage::repository::ensure_default_settings(&db)
                    .await
                    .map_err(|error| error.to_string())?;
                handle.manage(Arc::new(AppState::new(db)));
                Ok::<(), String>(())
            })?;

            setup_tray(app)?;
            scheduler::start_quota_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection_list,
            commands::connection_save,
            commands::connection_delete,
            commands::connection_test,
            commands::quota_get_latest,
            commands::quota_refresh_all,
            commands::window_show,
            commands::window_hide
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenBall");
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_orb = MenuItem::with_id(app, "show_orb", "显示余量球", true, None::<&str>)?;
    let open_main = MenuItem::with_id(app, "open_main", "打开管理", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_orb, &open_main, &refresh, &quit])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_orb" => crate::windows::show_window(app, "orb"),
            "open_main" => crate::windows::show_window(app, "main"),
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
        .build(app)?;
    Ok(())
}
