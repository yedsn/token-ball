use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// 进程级互斥锁，避免并发下载安装更新。
static INSTALL_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// 更新检查结果，前端据此渲染“关于/检查更新”面板。
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl UpdateInfo {
    fn up_to_date(current_version: &str) -> Self {
        Self {
            current_version: current_version.to_string(),
            available: false,
            version: None,
            date: None,
            notes: None,
        }
    }
}

/// 下载进度事件负载。
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: Option<f64>,
}

#[tauri::command]
pub fn updater_get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// 手动触发更新检查，成功时返回最新状态并通过 `updater://status` 通知前端。
#[tauri::command]
pub async fn updater_check(app: AppHandle) -> Result<UpdateInfo, String> {
    check_update(&app).await
}

/// 下载并安装更新，全程通过事件推进前端 UI，安装完成后自动重启。
#[tauri::command]
pub async fn updater_download_and_install(app: AppHandle) -> Result<(), String> {
    if INSTALL_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return Err("更新正在下载中，请勿重复操作".to_string());
    }

    let result = run_download_and_install(&app).await;

    INSTALL_IN_PROGRESS.store(false, Ordering::SeqCst);
    result
}

/// 单独提供重启入口，便于前端在异常后手动重启。
#[tauri::command]
pub fn updater_restart(app: AppHandle) {
    app.restart();
}

/// 检查更新核心逻辑，命令与启动自检共用。
pub async fn check_update(app: &AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();
    let updater = app.updater().map_err(|error| error.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => {
            let info = UpdateInfo {
                current_version,
                available: true,
                version: Some(update.version.clone()),
                date: update.date.map(|date| date.to_string()),
                notes: update.body.clone(),
            };
            let _ = app.emit("updater://status", &info);
            Ok(info)
        }
        Ok(None) => {
            let info = UpdateInfo::up_to_date(&current_version);
            let _ = app.emit("updater://status", &info);
            Ok(info)
        }
        Err(error) => {
            let message = error.to_string();
            let _ = app.emit("updater://failed", &message);
            Err(message)
        }
    }
}

/// 启动后静默自检：成功时通过 `updater://status` 通知前端，网络等瞬时错误只静默失败。
pub async fn run_startup_check(app: AppHandle) {
    if let Err(error) = check_update(&app).await {
        tracing::warn!("启动更新自检失败：{error}");
    }
}

async fn run_download_and_install(app: &AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "当前已是最新版本，无需更新".to_string())?;

    let _ = app.emit("updater://download-started", ());

    let progress_handle = app.clone();
    let finish_handle = app.clone();
    let mut total_downloaded: usize = 0;
    let result = update
        .download_and_install(
            move |chunk_length, total| {
                total_downloaded += chunk_length;
                let percent = total.map(|total| {
                    if total == 0 {
                        0.0
                    } else {
                        (total_downloaded as f64 / total as f64) * 100.0
                    }
                });
                let progress = DownloadProgress {
                    downloaded: total_downloaded as u64,
                    total,
                    percent,
                };
                let _ = progress_handle.emit("updater://download-progress", &progress);
            },
            move || {
                let _ = finish_handle.emit("updater://download-finished", ());
            },
        )
        .await;

    match result {
        Ok(()) => {
            let _ = app.emit("updater://installed", ());
            // NSIS/passive 安装器在返回后已替换可执行文件，重启以加载新版本。
            // restart() 直接退出进程（返回 !），无需再返回 Ok(())。
            app.restart()
        }
        Err(error) => {
            let message = error.to_string();
            let _ = app.emit("updater://failed", &message);
            Err(message)
        }
    }
}
