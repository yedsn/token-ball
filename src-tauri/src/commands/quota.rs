use std::sync::Arc;

use chrono::Utc;
use tauri::{AppHandle, State};

use crate::{
    app_state::AppState,
    error::AppResult,
    events,
    providers::{
        cliproxy::{mapper::map_auth_files, wham::enrich_codex_quotas, CliProxyClient},
        qianwen::QianwenClient,
        volcengine::VolcengineClient,
    },
    quota::{build_summary, ConnectionStatus, ProviderType, QuotaSummary},
    storage::repository,
};

#[tauri::command]
pub async fn quota_get_latest(state: State<'_, Arc<AppState>>) -> Result<QuotaSummary, String> {
    repository::load_summary(&state.db, false)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn quota_refresh_all(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<QuotaSummary, String> {
    refresh_all_internal(&app, &state).await.map_err(Into::into)
}

#[tauri::command]
pub async fn quota_refresh_connection(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<QuotaSummary, String> {
    events::emit_refresh_started(&app);
    let lock = {
        let mut locks = state.sync_locks.lock().await;
        locks
            .entry(connection_id.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _guard = lock.lock().await;

    let result = match sync_connection(&state, &connection_id).await {
        Ok(_) => repository::update_connection_status(
            &state.db,
            &connection_id,
            ConnectionStatus::Healthy,
            Some(Utc::now()),
        )
        .await
        .map_err(String::from),
        Err(error) => {
            let message = String::from(error);
            events::emit_provider_error(&app, &message);
            let _ = repository::update_connection_status(
                &state.db,
                &connection_id,
                ConnectionStatus::Degraded,
                None,
            )
            .await;
            Err(message)
        }
    };

    let summary = repository::load_summary(&state.db, result.is_err())
        .await
        .map_err(String::from)?;
    emit_connections_updated(&app, &state).await;
    events::emit_quota_updated(&app, &summary);
    events::emit_refresh_completed(&app, &summary);
    crate::tray::update_tray(&app, &summary).await;
    result.map(|_| summary)
}

pub async fn refresh_all_internal(
    app: &AppHandle,
    state: &Arc<AppState>,
) -> AppResult<QuotaSummary> {
    events::emit_refresh_started(app);
    let connections = repository::list_connections(&state.db).await?;
    let mut any_error = None;

    for connection in connections
        .into_iter()
        .filter(|connection| connection.enabled)
    {
        let lock = {
            let mut locks = state.sync_locks.lock().await;
            locks
                .entry(connection.id.clone())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;

        match sync_connection(state, &connection.id).await {
            Ok(_) => {
                repository::update_connection_status(
                    &state.db,
                    &connection.id,
                    ConnectionStatus::Healthy,
                    Some(Utc::now()),
                )
                .await?;
            }
            Err(error) => {
                let message = String::from(error);
                events::emit_provider_error(app, &message);
                any_error = Some(message);
                repository::update_connection_status(
                    &state.db,
                    &connection.id,
                    ConnectionStatus::Degraded,
                    None,
                )
                .await?;
            }
        }
    }

    let summary = repository::load_summary(&state.db, any_error.is_some()).await?;
    emit_connections_updated(app, state).await;
    events::emit_quota_updated(app, &summary);
    events::emit_refresh_completed(app, &summary);
    crate::tray::update_tray(app, &summary).await;
    Ok(summary)
}

async fn emit_connections_updated(app: &AppHandle, state: &Arc<AppState>) {
    if let Ok(connections) = repository::list_connections(&state.db).await {
        events::emit_connections_updated(app, &connections);
    }
}

async fn sync_connection(state: &Arc<AppState>, connection_id: &str) -> AppResult<()> {
    let (connection, key) = repository::get_connection_secret(&state.db, connection_id).await?;
    let accounts = match connection.provider_type {
        ProviderType::CliProxyApi => {
            let client = CliProxyClient::new(&connection.base_url, &key)?;
            let payload = client.auth_files().await?;
            let mut accounts = map_auth_files(&connection.id, &payload);
            enrich_codex_quotas(&client, &mut accounts).await?;
            accounts
        }
        ProviderType::Volcengine => {
            let client = VolcengineClient::new(&connection.base_url, &key)?;
            client.account_snapshot(&connection.id).await?
        }
        ProviderType::Qianwen => {
            let client = QianwenClient::new(&connection.base_url, &key)?;
            client.account_snapshot(&connection.id).await?
        }
    };
    repository::replace_connection_accounts(&state.db, connection_id, &accounts).await?;
    let _summary = build_summary(accounts, ConnectionStatus::Healthy, false);
    Ok(())
}
