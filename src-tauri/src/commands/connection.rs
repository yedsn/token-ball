use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    app_state::AppState,
    events,
    error::AppResult,
    providers::{cliproxy::CliProxyClient, qianwen::QianwenClient, volcengine::VolcengineClient},
    quota::{ConnectionInput, ProviderConnection, ProviderType},
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
