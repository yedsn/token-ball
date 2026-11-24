use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    storage::repository,
};

const SETTINGS_KEY: &str = "integration.phonebox";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct PhoneBoxSettings {
    pub enabled: bool,
    pub base_url: String,
}

impl Default for PhoneBoxSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "http://127.0.0.1:8765".into(),
        }
    }
}

pub fn normalize(settings: &PhoneBoxSettings) -> AppResult<PhoneBoxSettings> {
    let raw = settings.base_url.trim();
    let invalid = || {
        AppError::Message("PhoneBox 地址须为同机 HTTP 根地址，例如 http://127.0.0.1:8765".into())
    };
    let url = Url::parse(raw).map_err(|_| invalid())?;
    // Validate the literal authority too: URL parsing accepts aliases such as 127.1 or hex IPs.
    let authority = raw
        .strip_prefix("http://")
        .ok_or_else(invalid)?
        .split(['/', '?', '#'])
        .next()
        .ok_or_else(invalid)?;
    let suffix = &raw[7 + authority.len()..];
    if !matches!(suffix, "" | "/") {
        return Err(invalid());
    }
    let port = if let Some(rest) = authority.strip_prefix("[::1]") {
        rest
    } else if let Some(rest) = authority.strip_prefix("127.0.0.1") {
        rest
    } else {
        return Err(invalid());
    };
    if !port.is_empty()
        && (!port.starts_with(':')
            || port[1..].is_empty()
            || !port[1..].bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid());
    }
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || url.port_or_known_default() == Some(0)
        || raw.contains('\\')
    {
        return Err(invalid());
    }
    Ok(PhoneBoxSettings {
        enabled: settings.enabled,
        base_url: url.as_str().trim_end_matches('/').into(),
    })
}

pub async fn load(pool: &SqlitePool) -> AppResult<PhoneBoxSettings> {
    let settings = match repository::get_setting(pool, SETTINGS_KEY).await? {
        Some(value) => serde_json::from_str(&value)
            .map_err(|_| AppError::Message("PhoneBox 设置无效，请重新保存".into()))?,
        None => PhoneBoxSettings::default(),
    };
    normalize(&settings)
}

pub async fn save(pool: &SqlitePool, settings: &PhoneBoxSettings) -> AppResult<PhoneBoxSettings> {
    let settings = normalize(settings)?;
    let json = serde_json::to_string(&settings).map_err(|e| AppError::Message(e.to_string()))?;
    repository::set_setting(pool, SETTINGS_KEY, &json).await?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_literal_loopback_root_urls_are_accepted() {
        for raw in [
            "http://127.0.0.1:8765",
            "http://[::1]:8765/",
            "http://127.0.0.1:4567/",
            "http://127.0.0.1",
        ] {
            assert!(
                normalize(&PhoneBoxSettings {
                    base_url: raw.into(),
                    ..Default::default()
                })
                .is_ok(),
                "{raw}"
            );
        }
        for raw in [
            "http://localhost:8765",
            "https://127.0.0.1",
            "http://127.1",
            "http://2130706433",
            "http://127.0.0.1.evil.test",
            "http://192.168.1.2:8765",
            "http://127.0.0.1:0",
            "http://user@127.0.0.1",
            "http://127.0.0.1/api",
            "http://127.0.0.1?x=1",
            "http://127.0.0.1#frag",
            "http://127.0.0.1:99999",
            "http://127.0.0.1:/",
            "http://127.0.0.1/../",
            "http://[::1]/path",
        ] {
            // Dot paths must not be silently normalized into a root URL.
            let result = normalize(&PhoneBoxSettings {
                base_url: raw.into(),
                ..Default::default()
            });
            assert!(result.is_err(), "{raw}");
        }
    }

    #[tokio::test]
    async fn settings_are_opt_in_independent_and_persistent() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::storage::database::run_migrations_for_test(&pool)
            .await
            .unwrap();
        repository::ensure_default_settings(&pool).await.unwrap();
        let before = repository::get_setting(&pool, "display.quota")
            .await
            .unwrap();
        assert!(!load(&pool).await.unwrap().enabled);
        let settings = PhoneBoxSettings {
            enabled: true,
            base_url: "http://127.0.0.1:8765/".into(),
        };
        let saved = save(&pool, &settings).await.unwrap();
        assert_eq!(saved.base_url, "http://127.0.0.1:8765");
        assert_eq!(load(&pool).await.unwrap(), saved);
        assert_eq!(
            repository::get_setting(&pool, "display.quota")
                .await
                .unwrap(),
            before
        );
        assert_eq!(
            serde_json::from_str::<PhoneBoxSettings>("{}").unwrap(),
            PhoneBoxSettings::default()
        );
    }

    #[tokio::test]
    async fn persisted_settings_survive_database_reopen_and_invalid_data_is_repairable() {
        let path = std::env::temp_dir().join(format!(
            "tokenball-phonebox-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options.clone())
            .await
            .unwrap();
        crate::storage::database::run_migrations_for_test(&pool)
            .await
            .unwrap();
        let expected = save(
            &pool,
            &PhoneBoxSettings {
                enabled: true,
                base_url: "http://[::1]:9876/".into(),
            },
        )
        .await
        .unwrap();
        pool.close().await;
        let reopened = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        assert_eq!(load(&reopened).await.unwrap(), expected);
        repository::set_setting(&reopened, SETTINGS_KEY, "invalid")
            .await
            .unwrap();
        assert!(load(&reopened).await.is_err());
        save(&reopened, &PhoneBoxSettings::default()).await.unwrap();
        assert!(!load(&reopened).await.unwrap().enabled);
        reopened.close().await;
        std::fs::remove_file(&path).unwrap();
    }
}
