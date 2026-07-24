use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    quota::{
        build_summary, mask_secret, parse_account_status, parse_connection_status,
        parse_period_type, parse_quota_unit, ConnectionInput, ConnectionStatus, ProviderConnection,
        ProviderType, QuotaAccount, QuotaSummary, QuotaWindow, RequestActivity,
    },
};

pub async fn save_connection(
    pool: &SqlitePool,
    input: ConnectionInput,
) -> AppResult<ProviderConnection> {
    let now = Utc::now();
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let enabled = input.enabled.unwrap_or(true);

    sqlx::query(
        r#"
        INSERT INTO provider_connections
            (id, provider_type, display_name, base_url, management_key, enabled, status, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(id) DO UPDATE SET
            display_name = excluded.display_name,
            base_url = excluded.base_url,
            management_key = excluded.management_key,
            enabled = excluded.enabled,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(ProviderType::CliProxyApi.to_string())
    .bind(input.display_name)
    .bind(input.base_url)
    .bind(input.management_key)
    .bind(enabled as i64)
    .bind(ConnectionStatus::Unknown.to_string())
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    get_connection(pool, &id).await
}

pub async fn list_connections(pool: &SqlitePool) -> AppResult<Vec<ProviderConnection>> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_type, display_name, base_url, management_key, enabled, status,
               last_synced_at, created_at, updated_at
        FROM provider_connections
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_connection).collect()
}

pub async fn get_connection(pool: &SqlitePool, id: &str) -> AppResult<ProviderConnection> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_type, display_name, base_url, management_key, enabled, status,
               last_synced_at, created_at, updated_at
        FROM provider_connections
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    row_to_connection(row)
}

pub async fn get_connection_secret(
    pool: &SqlitePool,
    id: &str,
) -> AppResult<(ProviderConnection, String)> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_type, display_name, base_url, management_key, enabled, status,
               last_synced_at, created_at, updated_at
        FROM provider_connections
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    let key: String = row.try_get("management_key")?;
    Ok((row_to_connection(row)?, key))
}

pub async fn delete_connection(pool: &SqlitePool, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM provider_connections WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_connection_status(
    pool: &SqlitePool,
    id: &str,
    status: ConnectionStatus,
    last_synced_at: Option<DateTime<Utc>>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE provider_connections
        SET status = ?2, last_synced_at = COALESCE(?3, last_synced_at), updated_at = ?4
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .bind(status.to_string())
    .bind(last_synced_at.map(|dt| dt.to_rfc3339()))
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn replace_accounts(pool: &SqlitePool, accounts: &[QuotaAccount]) -> AppResult<()> {
    for account in accounts {
        let recent_requests = serde_json::to_string(&account.recent_requests)
            .map_err(|error| AppError::Message(error.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO provider_accounts
                (id, connection_id, external_id, display_name, masked_identifier, plan_name, status,
                 success_count, failed_count, recent_requests, subscription_until, chatgpt_account_id, last_synced_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(connection_id, external_id) DO UPDATE SET
                display_name = excluded.display_name,
                masked_identifier = excluded.masked_identifier,
                plan_name = excluded.plan_name,
                status = excluded.status,
                success_count = excluded.success_count,
                failed_count = excluded.failed_count,
                recent_requests = excluded.recent_requests,
                subscription_until = excluded.subscription_until,
                chatgpt_account_id = excluded.chatgpt_account_id,
                last_synced_at = excluded.last_synced_at
            "#,
        )
        .bind(&account.id)
        .bind(&account.connection_id)
        .bind(&account.external_id)
        .bind(&account.display_name)
        .bind(&account.masked_identifier)
        .bind(&account.plan_name)
        .bind(account.status.to_string())
        .bind(account.success_count)
        .bind(account.failed_count)
        .bind(recent_requests)
        .bind(account.subscription_until.map(|dt| dt.to_rfc3339()))
        .bind(&account.chatgpt_account_id)
        .bind(account.synced_at.to_rfc3339())
        .execute(pool)
        .await?;

        sqlx::query("DELETE FROM quota_snapshots WHERE account_id = ?1")
            .bind(&account.id)
            .execute(pool)
            .await?;

        let snapshot_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO quota_snapshots
                (id, account_id, status, critical_window_id, next_reset_at, collected_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&snapshot_id)
        .bind(&account.id)
        .bind(account.status.to_string())
        .bind(&account.critical_window_id)
        .bind(account.next_reset_at.map(|dt| dt.to_rfc3339()))
        .bind(account.synced_at.to_rfc3339())
        .execute(pool)
        .await?;

        for window in &account.windows {
            sqlx::query(
                r#"
                INSERT INTO quota_windows
                    (id, snapshot_id, name, period_type, period_seconds, total, used, remaining,
                     remaining_percent, unit, reset_at, is_active, is_current_constraint, data_source)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
            )
            .bind(&window.id)
            .bind(&snapshot_id)
            .bind(&window.name)
            .bind(window.period_type.to_string())
            .bind(window.period_seconds)
            .bind(window.total)
            .bind(window.used)
            .bind(window.remaining)
            .bind(window.remaining_percent)
            .bind(window.unit.to_string())
            .bind(window.reset_at.map(|dt| dt.to_rfc3339()))
            .bind(window.is_active as i64)
            .bind(window.is_current_constraint as i64)
            .bind(&window.data_source)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn load_accounts(pool: &SqlitePool) -> AppResult<Vec<QuotaAccount>> {
    let account_rows = sqlx::query(
        r#"
        SELECT a.id, a.connection_id, a.external_id, a.display_name, a.masked_identifier,
               a.plan_name, a.success_count, a.failed_count, a.recent_requests, a.subscription_until,
               a.chatgpt_account_id,
               s.status, s.critical_window_id, s.next_reset_at, s.collected_at
        FROM provider_accounts a
        LEFT JOIN quota_snapshots s ON s.account_id = a.id
        ORDER BY a.display_name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut accounts = Vec::new();
    for row in account_rows {
        let account_id: String = row.try_get("id")?;
        let snapshot_id: Option<String> =
            sqlx::query("SELECT id FROM quota_snapshots WHERE account_id = ?1 LIMIT 1")
                .bind(&account_id)
                .fetch_optional(pool)
                .await?
                .map(|snapshot| snapshot.try_get("id"))
                .transpose()?;
        let windows = if let Some(snapshot_id) = snapshot_id {
            load_windows(pool, &snapshot_id).await?
        } else {
            Vec::new()
        };
        let synced_at =
            parse_dt(row.try_get::<Option<String>, _>("collected_at")?).unwrap_or_else(Utc::now);

        accounts.push(QuotaAccount {
            id: account_id,
            connection_id: row.try_get("connection_id")?,
            external_id: row.try_get("external_id")?,
            display_name: row.try_get("display_name")?,
            masked_identifier: row.try_get("masked_identifier")?,
            plan_name: row.try_get("plan_name")?,
            status: parse_account_status(
                &row.try_get::<Option<String>, _>("status")?
                    .unwrap_or_default(),
            ),
            windows,
            critical_window_id: row.try_get("critical_window_id")?,
            next_reset_at: parse_dt(row.try_get("next_reset_at")?),
            success_count: row.try_get("success_count")?,
            failed_count: row.try_get("failed_count")?,
            recent_requests: parse_recent_requests(row.try_get("recent_requests")?),
            subscription_until: parse_dt(row.try_get("subscription_until")?),
            chatgpt_account_id: row.try_get("chatgpt_account_id")?,
            synced_at,
        });
    }
    Ok(accounts)
}

pub async fn load_summary(pool: &SqlitePool, stale: bool) -> AppResult<QuotaSummary> {
    let accounts = load_accounts(pool).await?;
    let status = list_connections(pool)
        .await?
        .first()
        .map(|connection| connection.status.clone())
        .unwrap_or(ConnectionStatus::Unknown);
    Ok(build_summary(accounts, status, stale))
}

pub async fn ensure_default_settings(pool: &SqlitePool) -> AppResult<()> {
    for (key, value) in [
        ("orb.size", "84"),
        ("orb.carouselIntervalMs", "4000"),
        ("sync.intervalSeconds", "180"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO NOTHING
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn get_setting(pool: &SqlitePool, key: &str) -> AppResult<Option<String>> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| row.try_get("value")).transpose()?)
}

async fn load_windows(pool: &SqlitePool, snapshot_id: &str) -> AppResult<Vec<QuotaWindow>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, period_type, period_seconds, total, used, remaining, remaining_percent,
               unit, reset_at, is_active, is_current_constraint, data_source
        FROM quota_windows
        WHERE snapshot_id = ?1
        ORDER BY remaining_percent ASC
        "#,
    )
    .bind(snapshot_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(QuotaWindow {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                period_type: parse_period_type(&row.try_get::<String, _>("period_type")?),
                period_seconds: row.try_get("period_seconds")?,
                total: row.try_get("total")?,
                used: row.try_get("used")?,
                remaining: row.try_get("remaining")?,
                remaining_percent: row.try_get("remaining_percent")?,
                unit: parse_quota_unit(&row.try_get::<String, _>("unit")?),
                reset_at: parse_dt(row.try_get("reset_at")?),
                is_active: row.try_get::<i64, _>("is_active")? == 1,
                is_current_constraint: row.try_get::<i64, _>("is_current_constraint")? == 1,
                data_source: row.try_get("data_source")?,
            })
        })
        .collect()
}

fn row_to_connection(row: sqlx::sqlite::SqliteRow) -> AppResult<ProviderConnection> {
    let management_key: String = row.try_get("management_key")?;
    Ok(ProviderConnection {
        id: row.try_get("id")?,
        provider_type: ProviderType::CliProxyApi,
        display_name: row.try_get("display_name")?,
        base_url: row.try_get("base_url")?,
        enabled: row.try_get::<i64, _>("enabled")? == 1,
        status: parse_connection_status(&row.try_get::<String, _>("status")?),
        last_synced_at: parse_dt(row.try_get("last_synced_at")?),
        created_at: parse_dt(row.try_get("created_at")?).unwrap_or_else(Utc::now),
        updated_at: parse_dt(row.try_get("updated_at")?).unwrap_or_else(Utc::now),
        masked_management_key: Some(mask_secret(&management_key)),
    })
}

fn parse_dt(value: Option<String>) -> Option<DateTime<Utc>> {
    value
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn parse_recent_requests(value: Option<String>) -> Vec<RequestActivity> {
    value
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}
