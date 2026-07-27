use std::{fs, path::PathBuf, sync::Arc};

use tauri::{AppHandle, State};

use crate::{
    app_state::AppState,
    error::AppResult,
    events,
    providers::{cliproxy::CliProxyClient, qianwen::QianwenClient, volcengine::VolcengineClient},
    quota::{
        ConfigBackup, ConfigBackupInfo, ConnectionInput, ExportConfigResult, ImportConfigResult,
        ProviderConnection, ProviderType,
    },
    storage::repository,
};

#[tauri::command]
pub async fn connection_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProviderConnection>, String> {
    repository::list_connections(&state.db)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn connection_save(
    state: State<'_, Arc<AppState>>,
    input: ConnectionInput,
) -> Result<ProviderConnection, String> {
    repository::save_connection(&state.db, input)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn connection_export_config_to_file(
    state: State<'_, Arc<AppState>>,
    file_path: String,
) -> Result<ExportConfigResult, String> {
    let file_path = PathBuf::from(file_path);
    if let Some(parent) = file_path.parent() {
        if !parent.is_dir() {
            return Err("导出位置不存在".to_string());
        }
    }

    let backup = repository::export_connections_backup(&state.db)
        .await
        .map_err::<String, _>(Into::into)?;
    let content = serde_json::to_string_pretty(&backup).map_err(|error| error.to_string())?;
    fs::write(&file_path, content).map_err(|error| error.to_string())?;

    Ok(ExportConfigResult {
        file_path: file_path.to_string_lossy().to_string(),
        exported_connections: backup.connections.len(),
    })
}

#[tauri::command]
pub async fn connection_import_config(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    backup: ConfigBackup,
) -> Result<ImportConfigResult, String> {
    let result = repository::import_connections_backup(&state.db, backup)
        .await
        .map_err::<String, _>(Into::into)?;
    if let Ok(connections) = repository::list_connections(&state.db).await {
        events::emit_connections_updated(&app, &connections);
    }
    Ok(result)
}

#[tauri::command]
pub async fn connection_read_config_backup(file_path: String) -> Result<ConfigBackupInfo, String> {
    let backup = read_config_backup_file(file_path)?;
    if backup.schema != "token-ball.connection-backup.v1" {
        return Err(format!("备份文件版本不受支持：{}", backup.schema));
    }
    Ok(ConfigBackupInfo {
        connection_count: backup.connections.len(),
    })
}

#[tauri::command]
pub async fn connection_import_config_from_file(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    file_path: String,
) -> Result<ImportConfigResult, String> {
    let backup = read_config_backup_file(file_path)?;
    let result = repository::import_connections_backup(&state.db, backup)
        .await
        .map_err::<String, _>(Into::into)?;
    if let Ok(connections) = repository::list_connections(&state.db).await {
        events::emit_connections_updated(&app, &connections);
    }
    Ok(result)
}

fn read_config_backup_file(file_path: String) -> Result<ConfigBackup, String> {
    let file_path = PathBuf::from(file_path);
    if !file_path.is_file() {
        return Err("备份文件不存在".to_string());
    }
    let content = fs::read_to_string(&file_path).map_err(|error| error.to_string())?;
    serde_json::from_str::<ConfigBackup>(&content).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn connection_delete(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    repository::delete_connection(&state.db, &id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn connection_set_enabled(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
    enabled: bool,
) -> Result<ProviderConnection, String> {
    let connection = repository::set_connection_enabled(&state.db, &id, enabled)
        .await
        .map_err::<String, _>(Into::into)?;
    if let Ok(connections) = repository::list_connections(&state.db).await {
        events::emit_connections_updated(&app, &connections);
    }
    Ok(connection)
}

#[tauri::command]
pub async fn connection_test(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    test_connection_internal(&state, &id)
        .await
        .map_err(Into::into)
}

pub async fn test_connection_internal(state: &Arc<AppState>, id: &str) -> AppResult<()> {
    let (connection, key) = repository::get_connection_secret(&state.db, id).await?;
    match connection.provider_type {
        ProviderType::CliProxyApi => {
            CliProxyClient::new(&connection.base_url, &key)?
                .test_connection()
                .await
        }
        ProviderType::Volcengine => {
            VolcengineClient::new(&connection.base_url, &key)?
                .test_connection()
                .await
        }
        ProviderType::Qianwen => {
            QianwenClient::new(&connection.base_url, &key)?
                .test_connection()
                .await
        }
    }
}
