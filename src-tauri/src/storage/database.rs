use std::fs;
use std::str::FromStr;

use sqlx::{sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, Executor, SqlitePool};

use crate::error::{AppError, AppResult};

pub async fn init_database() -> AppResult<SqlitePool> {
    let data_dir = dirs::data_local_dir()
        .ok_or(AppError::DataDirUnavailable)?
        .join("TokenBall");
    fs::create_dir_all(&data_dir).map_err(|error| AppError::Message(error.to_string()))?;
    let db_path = data_dir.join("tokenball.sqlite3");
    let url = format!("sqlite:{}", db_path.to_string_lossy());
    let options = SqliteConnectOptions::from_str(&url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    pool.execute("PRAGMA journal_mode = WAL;").await?;
    pool.execute("PRAGMA foreign_keys = ON;").await?;
    pool.execute("PRAGMA busy_timeout = 5000;").await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> AppResult<()> {
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS provider_connections (
            id TEXT PRIMARY KEY,
            provider_type TEXT NOT NULL,
            display_name TEXT NOT NULL,
            base_url TEXT NOT NULL,
            management_key TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            status TEXT NOT NULL,
            last_synced_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .await?;

    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS provider_accounts (
            id TEXT PRIMARY KEY,
            connection_id TEXT NOT NULL,
            external_id TEXT NOT NULL,
            display_name TEXT NOT NULL,
            masked_identifier TEXT,
            plan_name TEXT NOT NULL,
            status TEXT NOT NULL,
            success_count INTEGER,
            failed_count INTEGER,
            recent_requests TEXT NOT NULL DEFAULT '[]',
            subscription_until TEXT,
            chatgpt_account_id TEXT,
            last_synced_at TEXT,
            UNIQUE(connection_id, external_id),
            FOREIGN KEY(connection_id) REFERENCES provider_connections(id) ON DELETE CASCADE
        );
        "#,
    )
    .await?;

    for statement in [
        "ALTER TABLE provider_accounts ADD COLUMN success_count INTEGER",
        "ALTER TABLE provider_accounts ADD COLUMN failed_count INTEGER",
        "ALTER TABLE provider_accounts ADD COLUMN recent_requests TEXT NOT NULL DEFAULT '[]'",
        "ALTER TABLE provider_accounts ADD COLUMN subscription_until TEXT",
        "ALTER TABLE provider_accounts ADD COLUMN chatgpt_account_id TEXT",
    ] {
        let _ = pool.execute(statement).await;
    }

    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS quota_snapshots (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            status TEXT NOT NULL,
            critical_window_id TEXT,
            next_reset_at TEXT,
            collected_at TEXT NOT NULL,
            FOREIGN KEY(account_id) REFERENCES provider_accounts(id) ON DELETE CASCADE
        );
        "#,
    )
    .await?;

    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS quota_windows (
            id TEXT PRIMARY KEY,
            snapshot_id TEXT NOT NULL,
            name TEXT NOT NULL,
            period_type TEXT NOT NULL,
            period_seconds INTEGER,
            total REAL,
            used REAL,
            remaining REAL,
            remaining_percent REAL,
            unit TEXT NOT NULL,
            reset_at TEXT,
            is_active INTEGER NOT NULL,
            is_current_constraint INTEGER NOT NULL,
            data_source TEXT NOT NULL,
            FOREIGN KEY(snapshot_id) REFERENCES quota_snapshots(id) ON DELETE CASCADE
        );
        "#,
    )
    .await?;

    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .await?;

    Ok(())
}
