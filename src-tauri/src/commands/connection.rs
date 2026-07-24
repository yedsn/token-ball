use std::sync::Arc;

use tauri::State;

use crate::{
    app_state::AppState,
    error::AppResult,
    providers::cliproxy::CliProxyClient,
    quota::{ConnectionInput, ProviderConnection},
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
pub async fn connection_test(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    test_connection_internal(&state, &id)
        .await
        .map_err(Into::into)
}

pub async fn test_connection_internal(state: &Arc<AppState>, id: &str) -> AppResult<()> {
    let (connection, key) = repository::get_connection_secret(&state.db, id).await?;
    let client = CliProxyClient::new(&connection.base_url, &key)?;
    client.test_connection().await
}
