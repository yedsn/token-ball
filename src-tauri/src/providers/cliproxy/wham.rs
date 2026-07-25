use chrono::{DateTime, TimeZone, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    quota::{AccountStatus, PeriodType, QuotaAccount, QuotaUnit, QuotaWindow},
};

use super::CliProxyClient;

const WHAM_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const WINDOW_5H_SECONDS: i64 = 5 * 60 * 60;
const WINDOW_7D_SECONDS: i64 = 7 * 24 * 60 * 60;

pub async fn enrich_codex_quotas(
    client: &CliProxyClient,
    accounts: &mut [QuotaAccount],
) -> AppResult<()> {
    for account in accounts {
        if !is_codex_account(account) {
            continue;
        }
        if account.external_id.trim().is_empty() {
            continue;
        }
        if let Some(account_id) = chatgpt_account_id(account) {
            match fetch_wham_usage(client, &account.external_id, &account_id).await {
                Ok(payload) => apply_wham_usage(account, &payload),
                Err(error) => {
                    let error_str = error.to_string();
                    let is_service_unavailable =
                        error_str.contains("503") || error_str.contains("Service Unavailable");
                    if account.windows.is_empty() && !is_service_unavailable {
                        account.status = AccountStatus::Warning;
                    }
                    if is_service_unavailable {
                        tracing::debug!(account = %account.display_name, error = %error_str, "wham service unavailable, keeping existing status");
                    } else {
                        tracing::warn!(account = %account.display_name, error = %error_str, "codex quota query failed");
                    }
                }
            }
        }
    }
    Ok(())
}

async fn fetch_wham_usage(
    client: &CliProxyClient,
    auth_index: &str,
    account_id: &str,
) -> AppResult<Value> {
    let response = client
        .api_call(
            auth_index,
            "GET",
            WHAM_USAGE_URL,
            json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json",
                "User-Agent": "codex_cli_rs/0.76.0 (Debian 13.0.0; x86_64) WindowsTerminal",
                "Chatgpt-Account-Id": account_id,
            }),
            None,
        )
        .await?;

    let status = integer_field(&response, &["status_code", "statusCode"]).unwrap_or(0);
    if !(200..300).contains(&status) {
        return Err(AppError::Message(format!(
            "wham usage HTTP {}：{}",
            status,
            body_text(response.get("body"))
        )));
    }

    parse_body(response.get("body")).ok_or(AppError::InvalidResponse)
}

fn apply_wham_usage(account: &mut QuotaAccount, payload: &Value) {
    if let Some(plan) = string_field(payload, &["plan_type", "planType"]) {
        account.plan_name = plan;
    }

    let mut windows = parse_codex_windows(&account.external_id, payload);
    windows.extend(parse_additional_windows(&account.external_id, payload));
    if windows.is_empty() {
        return;
    }

    let critical_id = windows
        .iter()
        .filter(|window| window.remaining_percent.is_some())
        .min_by(|left, right| {
            left.remaining_percent
                .unwrap_or(100.0)
                .partial_cmp(&right.remaining_percent.unwrap_or(100.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|window| window.id.clone());
    for window in &mut windows {
        window.is_current_constraint = critical_id.as_ref() == Some(&window.id);
    }

    account.next_reset_at = windows.iter().filter_map(|window| window.reset_at).min();
    account.critical_window_id = critical_id;
    account.status = status_from_windows(&windows);
    account.windows = windows;
}

fn parse_codex_windows(account_key: &str, payload: &Value) -> Vec<QuotaWindow> {
    let Some(rate_limit) = object_field(payload, &["rate_limit", "rateLimit"]) else {
        return Vec::new();
    };
    let (five_hour, weekly) = find_quota_windows(rate_limit);
    let limit_reached = bool_field(rate_limit, &["limit_reached", "limitReached"]);
    let allowed = bool_field(rate_limit, &["allowed"]);
    let mut windows = Vec::new();
    if let Some(window) = build_window(
        account_key,
        "code-5h",
        "5h",
        five_hour,
        limit_reached,
        allowed,
    ) {
        windows.push(window);
    }
    if let Some(window) = build_window(account_key, "code-7d", "7d", weekly, limit_reached, allowed)
    {
        windows.push(window);
    }
    windows
}

fn parse_additional_windows(account_key: &str, payload: &Value) -> Vec<QuotaWindow> {
    let Some(items) = payload
        .get("additional_rate_limits")
        .or_else(|| payload.get("additionalRateLimits"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut windows = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(rate_limit) = object_field(item, &["rate_limit", "rateLimit"]) else {
            continue;
        };
        let name = string_field(
            item,
            &[
                "limit_name",
                "limitName",
                "metered_feature",
                "meteredFeature",
            ],
        )
        .unwrap_or_else(|| format!("additional-{}", index + 1));
        let primary = object_field(rate_limit, &["primary_window", "primaryWindow"]);
        let secondary = object_field(rate_limit, &["secondary_window", "secondaryWindow"]);
        let limit_reached = bool_field(rate_limit, &["limit_reached", "limitReached"]);
        let allowed = bool_field(rate_limit, &["allowed"]);
        if let Some(window) = build_window(
            account_key,
            &format!("{}-primary", name),
            &format!("{} 5h", name),
            primary,
            limit_reached,
            allowed,
        ) {
            windows.push(window);
        }
        if let Some(window) = build_window(
            account_key,
            &format!("{}-secondary", name),
            &format!("{} 7d", name),
            secondary,
            limit_reached,
            allowed,
        ) {
            windows.push(window);
        }
    }
    windows
}

fn find_quota_windows(rate_limit: &Value) -> (Option<&Value>, Option<&Value>) {
    let primary = object_field(rate_limit, &["primary_window", "primaryWindow"]);
    let secondary = object_field(rate_limit, &["secondary_window", "secondaryWindow"]);
    let mut five_hour = None;
    let mut weekly = None;
    for candidate in [primary, secondary].into_iter().flatten() {
        let duration = number_field(candidate, &["limit_window_seconds", "limitWindowSeconds"]);
        if duration == Some(WINDOW_5H_SECONDS as f64) && five_hour.is_none() {
            five_hour = Some(candidate);
        }
        if duration == Some(WINDOW_7D_SECONDS as f64) && weekly.is_none() {
            weekly = Some(candidate);
        }
    }
    let fallback_five_hour = primary.filter(|window| {
        number_field(window, &["limit_window_seconds", "limitWindowSeconds"])
            != Some(WINDOW_7D_SECONDS as f64)
    });
    let fallback_weekly = secondary.or_else(|| {
        primary.filter(|window| {
            number_field(window, &["limit_window_seconds", "limitWindowSeconds"])
                == Some(WINDOW_7D_SECONDS as f64)
        })
    });
    (five_hour.or(fallback_five_hour), weekly.or(fallback_weekly))
}

fn build_window(
    account_key: &str,
    id: &str,
    name: &str,
    window: Option<&Value>,
    limit_reached: Option<bool>,
    allowed: Option<bool>,
) -> Option<QuotaWindow> {
    let window = window?;
    let used_percent = deduce_used_percent(window, limit_reached, allowed);
    let remaining_percent = used_percent.map(|used| clamp(100.0 - used, 0.0, 100.0));
    Some(QuotaWindow {
        id: Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("wham:{}:{}", account_key, id).as_bytes(),
        )
        .to_string(),
        name: name.to_string(),
        period_type: PeriodType::Rolling,
        period_seconds: number_field(window, &["limit_window_seconds", "limitWindowSeconds"])
            .map(|value| value as i64),
        total: Some(100.0),
        used: used_percent,
        remaining: remaining_percent,
        remaining_percent,
        unit: QuotaUnit::Percent,
        reset_at: reset_at(window),
        is_active: true,
        is_current_constraint: false,
        data_source: "chatgpt.wham_usage".to_string(),
    })
}

fn deduce_used_percent(
    window: &Value,
    limit_reached: Option<bool>,
    allowed: Option<bool>,
) -> Option<f64> {
    if let Some(used) = number_field(window, &["used_percent", "usedPercent"]) {
        return Some(clamp(used, 0.0, 100.0));
    }
    let exhausted_hint = limit_reached.unwrap_or(false) || allowed == Some(false);
    if exhausted_hint && reset_at(window).is_some() {
        return Some(100.0);
    }
    None
}

fn status_from_windows(windows: &[QuotaWindow]) -> AccountStatus {
    let lowest = windows
        .iter()
        .filter_map(|window| window.remaining_percent)
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    match lowest {
        Some(value) if value <= 0.0 => AccountStatus::Exhausted,
        Some(value) if value <= 30.0 => AccountStatus::Warning,
        Some(_) => AccountStatus::Available,
        None => AccountStatus::Unknown,
    }
}

fn chatgpt_account_id(account: &QuotaAccount) -> Option<String> {
    account
        .chatgpt_account_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_codex_account(account: &QuotaAccount) -> bool {
    account.plan_name.to_ascii_lowercase().contains("codex")
        || matches!(
            account.status,
            AccountStatus::Available
                | AccountStatus::Warning
                | AccountStatus::Cooldown
                | AccountStatus::Unknown
        )
}

fn parse_body(value: Option<&Value>) -> Option<Value> {
    match value? {
        Value::Object(_) => value.cloned(),
        Value::String(raw) => serde_json::from_str(raw).ok(),
        _ => None,
    }
}

fn body_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(raw)) => raw.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn object_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter()
        .find_map(|key| value.get(*key).filter(|item| item.is_object()))
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str).map(str::to_string))
}

fn number_field(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_f64))
}

fn integer_field(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_i64))
}

fn bool_field(value: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_bool))
}

fn reset_at(window: &Value) -> Option<DateTime<Utc>> {
    if let Some(timestamp) = integer_field(window, &["reset_at", "resetAt"]) {
        return Utc.timestamp_opt(timestamp, 0).single();
    }
    if let Some(seconds) = number_field(window, &["reset_after_seconds", "resetAfterSeconds"]) {
        return Some(Utc::now() + chrono::Duration::seconds(seconds as i64));
    }
    None
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use crate::providers::cliproxy::{mapper::map_auth_files, CliProxyClient};
    use serde_json::json;

    use super::parse_codex_windows;

    #[test]
    fn parses_wham_windows() {
        let windows = parse_codex_windows(
            "auth-1",
            &json!({
                "rate_limit": {
                    "primary_window": {
                        "used_percent": 20,
                        "limit_window_seconds": 18000,
                        "reset_after_seconds": 3600
                    },
                    "secondary_window": {
                        "used_percent": 35,
                        "limit_window_seconds": 604800,
                        "reset_after_seconds": 7200
                    }
                }
            }),
        );
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].name, "5h");
        assert_eq!(windows[0].remaining_percent, Some(80.0));
        assert_eq!(windows[1].name, "7d");
        assert_eq!(windows[1].remaining_percent, Some(65.0));
    }

    #[test]
    fn primary_weekly_window_is_not_duplicated_as_five_hour() {
        let windows = parse_codex_windows(
            "auth-1",
            &json!({
                "rate_limit": {
                    "allowed": true,
                    "limit_reached": false,
                    "primary_window": {
                        "used_percent": 48,
                        "limit_window_seconds": 604800,
                        "reset_after_seconds": 517833
                    },
                    "secondary_window": null
                }
            }),
        );
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].name, "7d");
        assert_eq!(windows[0].remaining_percent, Some(52.0));
    }

    #[tokio::test]
    async fn live_saved_connection_can_fetch_codex_quota_when_enabled() {
        if std::env::var("TOKENBALL_LIVE_CPA_TEST").ok().as_deref() != Some("1") {
            return;
        }

        let db_path = dirs::data_local_dir()
            .expect("local data dir")
            .join("TokenBall")
            .join("tokenball.sqlite3");
        let url = format!("sqlite:{}", db_path.to_string_lossy());
        let pool = sqlx::SqlitePool::connect(&url).await.expect("connect db");
        let row: (String, String) = sqlx::query_as(
            "SELECT base_url, management_key FROM provider_connections ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .expect("saved connection");

        let client = CliProxyClient::new(&row.0, &row.1).expect("client");
        let payload = client.auth_files().await.expect("auth files");
        let mut accounts = map_auth_files("live", &payload);
        super::enrich_codex_quotas(&client, &mut accounts)
            .await
            .expect("quota enrich");
        assert!(accounts.iter().any(|account| {
            account
                .windows
                .iter()
                .any(|window| window.remaining_percent.is_some())
        }));
    }
}
