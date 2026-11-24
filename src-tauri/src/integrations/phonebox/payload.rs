use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::{
    error::{AppError, AppResult},
    quota::{
        AccountStatus, ConnectionStatus, DisplaySettings, PeriodType, ProviderConnection,
        QuotaAccount, QuotaWindow,
    },
    storage::repository,
};

pub const MAX_BYTES: usize = 60 * 1024;

pub async fn load_payload(pool: &SqlitePool) -> AppResult<Vec<u8>> {
    // Keep missing stored timestamps distinct from the desktop model's now() fallback.
    let connections = repository::list_connections(pool).await?;
    let (accounts, collected) = repository::load_accounts_with_collection_times(pool).await?;
    let settings = repository::load_display_settings(pool).await?;
    encode(build_payload(
        &connections,
        &accounts,
        &collected,
        &settings,
        Utc::now(),
    ))
}

#[cfg(test)]
fn parse_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|d| d.with_timezone(&Utc))
        .filter(|d| d.timestamp() > 0)
}

fn opaque(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}
fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v >= 0.0)
}
fn percent(value: Option<f64>) -> Option<f64> {
    finite(value).filter(|v| *v <= 100.0)
}
fn date(value: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    value.filter(|d| d.timestamp() > 0)
}

fn period(window: &QuotaWindow) -> Option<&'static str> {
    match window.period_type {
        PeriodType::Daily => return None,
        PeriodType::Weekly => return Some("weekly"),
        PeriodType::Monthly => return Some("monthly"),
        _ => {}
    }
    match window.period_seconds {
        Some(18000) => return Some("fiveHour"),
        Some(604800) => return Some("weekly"),
        Some(2592000) => return Some("monthly"),
        _ => {}
    }
    let text = format!("{} {}", window.id, window.name).to_lowercase();
    // Match numeric aliases only at digit boundaries, like quotaPeriod.ts (never 15h/17d/130d).
    let numeric = |number: &str, suffix: &str| {
        text.match_indices(number).any(|(i, _)| {
            let before = text[..i].chars().next_back();
            let rest = text[i + number.len()..].trim_start();
            let Some(after) = rest.strip_prefix(suffix) else {
                return false;
            };
            !before.is_some_and(|c| c.is_ascii_digit())
                && (suffix == "小时"
                    || !after
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_'))
        })
    };
    if text.contains("five-hour") || numeric("5", "h") || numeric("5", "小时") {
        Some("fiveHour")
    } else if text.contains("week")
        || text.contains("每周")
        || text
            .split_whitespace()
            .collect::<String>()
            .contains("近1周")
        || numeric("7", "d")
    {
        Some("weekly")
    } else if text.contains("month")
        || text.contains("每月")
        || text
            .split_whitespace()
            .collect::<String>()
            .contains("近1月")
        || numeric("30", "d")
    {
        Some("monthly")
    } else {
        None
    }
}

pub fn build_payload(
    connections: &[ProviderConnection],
    accounts: &[QuotaAccount],
    collected: &HashMap<String, Option<DateTime<Utc>>>,
    settings: &DisplaySettings,
    now: DateTime<Utc>,
) -> Value {
    let connections: HashMap<_, _> = connections
        .iter()
        .filter(|c| c.enabled)
        .map(|c| (c.id.as_str(), c))
        .collect();
    let grace = settings
        .sync_interval_seconds
        .saturating_add(if settings.random_sync_delay_enabled {
            settings.random_sync_delay_max_seconds
        } else {
            0
        })
        .saturating_add(300)
        .max(300)
        .min(i64::MAX as u64) as i64;
    let mut rows = Vec::new();
    let mut values = Vec::new();
    let mut available = 0;
    let mut any_stale = false;
    for account in accounts {
        let Some(connection) = connections.get(account.connection_id.as_str()) else {
            continue;
        };
        let critical = account
            .critical_window_id
            .as_ref()
            .and_then(|id| account.windows.iter().find(|w| &w.id == id))
            .and_then(|w| percent(w.remaining_percent));
        if let Some(v) = critical {
            values.push(v);
        }
        if matches!(
            account.status,
            AccountStatus::Available | AccountStatus::Warning
        ) {
            available += 1;
        }
        let collected_at = collected
            .get(&account.id)
            .copied()
            .flatten()
            .and_then(|d| date(Some(d)));
        let stale_at = collected_at.and_then(|d| {
            Duration::try_seconds(grace).and_then(|delta| d.checked_add_signed(delta))
        });
        let stale = connection.status != ConnectionStatus::Healthy
            || collected_at.is_none_or(|d| d > now + Duration::seconds(300))
            || stale_at.is_none_or(|d| d <= now);
        any_stale |= stale;
        let mut periods = json!({"fiveHour":null, "weekly":null, "monthly":null});
        for window in &account.windows {
            if let Some(key) = period(window) {
                if periods[key].is_null() {
                    periods[key] = json!({"remainingPercent":percent(window.remaining_percent), "total":finite(window.total), "used":finite(window.used), "remaining":finite(window.remaining), "unit":window.unit, "resetAt":date(window.reset_at)});
                }
            }
        }
        let id = opaque(&account.id);
        let instance = opaque(&connection.id);
        rows.push(json!({
            "id":id, "alias":format!("账号 {}", &id[..8]), "provider":connection.provider_type,
            "displayName":account.display_name.chars().filter(|c| !c.is_control()).take(160).collect::<String>(),
            "priorityResetAt":priority_reset(account),
            "instanceAlias":format!("实例 {}", &instance[..8]),
            "planName":account.plan_name.chars().filter(|c| !c.is_control()).take(80).collect::<String>(),
            "status":account.status, "connectionStatus":connection.status, "collectedAt":collected_at,
            "staleAt":stale_at, "stale":stale, "subscriptionUntil":date(account.subscription_until),
            "criticalRemainingPercent":critical, "periods":periods
        }));
    }
    // Keep the most urgent rows when the payload budget requires truncation.
    // The page resolves name ties with the tray's zh-CN localeCompare.
    rows.sort_by(|a, b| {
        let time = |row: &Value, key: &str| {
            row[key]
                .as_str()
                .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        };
        let nullable = |left: Option<DateTime<chrono::FixedOffset>>,
                        right: Option<DateTime<chrono::FixedOffset>>| {
            match (left, right) {
                (Some(l), Some(r)) => l.cmp(&r),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        };
        nullable(time(a, "subscriptionUntil"), time(b, "subscriptionUntil"))
            .then_with(|| nullable(time(a, "priorityResetAt"), time(b, "priorityResetAt")))
            .then_with(|| a["displayName"].as_str().cmp(&b["displayName"].as_str()))
            .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    let count = rows.len();
    let sum: f64 = values.iter().sum();
    let average = (!values.is_empty()).then(|| sum / values.len() as f64);
    let equivalent = (!values.is_empty()).then_some(sum / 100.0);
    let level = if any_stale {
        "warn"
    } else if average.is_some_and(|v| v <= 10.0) {
        "error"
    } else if average.is_none_or(|v| v <= 30.0) {
        "warn"
    } else {
        "ok"
    };
    json!({
        "schema":"tokenball.phonebox.v1", "title":"TokenBall",
        "value":average.map_or(json!("--"), |v| json!((v * 10.0).round() / 10.0)),
        "unit":"%", "progress":average.map(|v| v / 100.0), "level":level,
        "sub":format!("{} / {} 可用 · {} 个已知额度 · {}", available, count, values.len(), if any_stale { "含缓存，详见账号采集时间" } else { "均值口径，详见账号采集时间" }),
        "pushedAt":now, "heartbeatTimeoutSeconds":180,
        "summary":{"accountCount":count, "availableAccountCount":available, "knownQuotaAccountCount":values.len(), "averageRemainingPercent":average, "equivalentAccounts":equivalent, "stale":any_stale},
        "includedAccountCount":count, "omittedAccountCount":0, "accounts":rows
    })
}

// Mirrors HoverPanelView.primaryExpiryWindow / balanceResetTime. This is only a
// sorting hint: never substitute it for an individual period's resetAt.
fn priority_reset(account: &QuotaAccount) -> Option<DateTime<Utc>> {
    let monthly = account
        .windows
        .iter()
        .find(|w| w.reset_at.is_some() && period(w) == Some("monthly"));
    let mut highest: Option<&QuotaWindow> = None;
    let priority = |w: &QuotaWindow| match period(w) {
        Some("monthly") => 2592000.0,
        Some("weekly") => 604800.0,
        Some("fiveHour") => 18000.0,
        _ => w
            .period_seconds
            .map(|n| n as f64)
            .or(w.total)
            .unwrap_or(0.0),
    };
    for window in account.windows.iter().filter(|w| w.reset_at.is_some()) {
        if highest.is_none_or(|previous| priority(window) > priority(previous)) {
            highest = Some(window);
        }
    }
    date(
        monthly
            .or(highest)
            .and_then(|w| w.reset_at)
            .or(account.next_reset_at),
    )
}

pub fn encode(mut payload: Value) -> AppResult<Vec<u8>> {
    loop {
        let bytes = serde_json::to_vec(&payload)
            .map_err(|_| AppError::Message("额度展示数据编码失败".into()))?;
        if bytes.len() <= MAX_BYTES {
            return Ok(bytes);
        }
        let rows = payload["accounts"].as_array_mut().unwrap();
        if rows.pop().is_none() {
            return Err(AppError::Message("额度展示汇总超出容量".into()));
        }
        let included = rows.len();
        payload["includedAccountCount"] = json!(included);
        payload["omittedAccountCount"] =
            json!(payload["summary"]["accountCount"].as_u64().unwrap() - included as u64);
    }
}

#[cfg(test)]
mod tests;
