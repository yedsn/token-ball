use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::quota::{
    mask_secret, select_critical_window, AccountStatus, PeriodType, QuotaAccount, QuotaUnit,
    QuotaWindow, RequestActivity,
};

pub fn map_auth_files(connection_id: &str, payload: &Value) -> Vec<QuotaAccount> {
    let values = extract_array(payload);
    let now = Utc::now();
    values
        .into_iter()
        .filter(|item| auth_file_enabled(item))
        .enumerate()
        .map(|(index, item)| {
            let external_id = string_field(
                item,
                &[
                    "auth_index",
                    "authIndex",
                    "id",
                    "authFileId",
                    "auth_file_id",
                    "path",
                    "email",
                ],
            )
            .unwrap_or_else(|| format!("account-{}", index + 1));
            let display_name = string_field(
                item,
                &[
                    "label",
                    "displayName",
                    "display_name",
                    "name",
                    "email",
                    "account",
                ],
            )
            .unwrap_or_else(|| format!("Codex 账号 {}", index + 1));
            let status = map_status(
                string_field(item, &["status", "state"]).as_deref(),
                bool_field(item, &["disabled"]),
                bool_field(item, &["unavailable"]),
            );
            let mut windows = map_windows(item, &external_id);
            let critical_id = select_critical_window(&windows).map(|window| window.id.clone());
            for window in &mut windows {
                window.is_current_constraint = critical_id.as_ref() == Some(&window.id);
            }
            let next_reset_at = windows.iter().filter_map(|window| window.reset_at).min();

            QuotaAccount {
                id: stable_id(connection_id, &external_id),
                connection_id: connection_id.to_string(),
                external_id,
                display_name,
                masked_identifier: string_field(item, &["email", "account", "identifier"])
                    .map(|value| mask_secret(&value)),
                plan_name: string_field(item, &["plan", "planName", "plan_name"])
                    .unwrap_or_else(|| "Codex Plus".to_string()),
                status,
                windows,
                critical_window_id: critical_id,
                next_reset_at,
                success_count: integer_field(item, &["success", "success_count", "successCount"]),
                failed_count: integer_field(item, &["failed", "failed_count", "failedCount"]),
                recent_requests: map_recent_requests(item),
                subscription_until: subscription_until(item),
                chatgpt_account_id: item.get("id_token").and_then(|token| {
                    string_field(
                        token,
                        &[
                            "chatgpt_account_id",
                            "account_id",
                            "accountId",
                            "https://api.openai.com/auth.chatgpt_account_id",
                        ],
                    )
                }),
                synced_at: now,
            }
        })
        .collect()
}

fn auth_file_enabled(item: &Value) -> bool {
    if bool_field(item, &["disabled"]).unwrap_or(false) {
        return false;
    }
    if !bool_field(item, &["enabled"]).unwrap_or(true) {
        return false;
    }

    !matches!(
        string_field(item, &["status", "state"])
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "disabled"
    )
}

fn extract_array(payload: &Value) -> Vec<&Value> {
    if let Some(array) = payload.as_array() {
        return array.iter().collect();
    }
    for key in [
        "data",
        "files",
        "authFiles",
        "auth_files",
        "accounts",
        "items",
    ] {
        if let Some(array) = payload.get(key).and_then(Value::as_array) {
            return array.iter().collect();
        }
    }
    Vec::new()
}

fn map_windows(item: &Value, external_id: &str) -> Vec<QuotaWindow> {
    let mut windows = Vec::new();
    for key in [
        "windows",
        "quotaWindows",
        "quota_windows",
        "limits",
        "quotas",
    ] {
        if let Some(array) = item.get(key).and_then(Value::as_array) {
            for (index, value) in array.iter().enumerate() {
                windows.push(map_window(value, external_id, index));
            }
        }
    }
    if windows.is_empty() {
        let percent = number_field(
            item,
            &[
                "remainingPercent",
                "remaining_percent",
                "percent",
                "quotaPercent",
            ],
        );
        if percent.is_some() {
            windows.push(QuotaWindow {
                id: Uuid::new_v4().to_string(),
                name: "当前窗口".to_string(),
                period_type: PeriodType::Unknown,
                period_seconds: None,
                total: number_field(item, &["total", "limit"]),
                used: number_field(item, &["used"]),
                remaining: number_field(item, &["remaining"]),
                remaining_percent: percent.map(normalize_percent),
                unit: QuotaUnit::Percent,
                reset_at: date_field(
                    item,
                    &["resetAt", "reset_at", "nextResetAt", "next_reset_at"],
                ),
                is_active: true,
                is_current_constraint: false,
                data_source: "cliproxy.auth_files".to_string(),
            });
        }
    }
    windows
}

fn map_window(value: &Value, external_id: &str, index: usize) -> QuotaWindow {
    QuotaWindow {
        id: Uuid::new_v4().to_string(),
        name: string_field(value, &["name", "label", "period"])
            .unwrap_or_else(|| format!("{} 窗口 {}", external_id, index + 1)),
        period_type: PeriodType::Unknown,
        period_seconds: number_field(value, &["periodSeconds", "period_seconds"]).map(|n| n as i64),
        total: number_field(value, &["total", "limit"]),
        used: number_field(value, &["used"]),
        remaining: number_field(value, &["remaining"]),
        remaining_percent: number_field(
            value,
            &["remainingPercent", "remaining_percent", "percent"],
        )
        .map(normalize_percent),
        unit: QuotaUnit::Percent,
        reset_at: date_field(
            value,
            &["resetAt", "reset_at", "nextResetAt", "next_reset_at"],
        ),
        is_active: bool_field(value, &["isActive", "is_active", "active"]).unwrap_or(true),
        is_current_constraint: false,
        data_source: "cliproxy.auth_files".to_string(),
    }
}

fn map_recent_requests(item: &Value) -> Vec<RequestActivity> {
    item.get("recent_requests")
        .or_else(|| item.get("recentRequests"))
        .and_then(Value::as_array)
        .map(|requests| {
            requests
                .iter()
                .filter_map(|request| {
                    Some(RequestActivity {
                        time: string_field(request, &["time", "window"])?,
                        success: integer_field(
                            request,
                            &["success", "success_count", "successCount"],
                        )
                        .unwrap_or(0),
                        failed: integer_field(request, &["failed", "failed_count", "failedCount"])
                            .unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn integer_field(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_i64))
}

fn map_status(
    value: Option<&str>,
    disabled: Option<bool>,
    unavailable: Option<bool>,
) -> AccountStatus {
    if disabled.unwrap_or(false) {
        return AccountStatus::Disabled;
    }
    if unavailable.unwrap_or(false) {
        return AccountStatus::Cooldown;
    }

    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "available" | "ok" | "active" => AccountStatus::Available,
        "warning" => AccountStatus::Warning,
        "cooldown" | "cooling" => AccountStatus::Cooldown,
        "exhausted" | "empty" => AccountStatus::Exhausted,
        "disabled" => AccountStatus::Disabled,
        "auth_expired" | "expired" => AccountStatus::AuthExpired,
        "offline" => AccountStatus::Offline,
        "error" | "failed" => AccountStatus::Error,
        _ => AccountStatus::Unknown,
    }
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str).map(str::to_string))
}

fn number_field(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_f64))
}

fn bool_field(value: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_bool))
}

fn subscription_until(item: &Value) -> Option<DateTime<Utc>> {
    const TOKEN_KEYS: &[&str] = &[
        "chatgpt_subscription_active_until",
        "subscription_until",
        "subscriptionUntil",
        "expires_at",
        "expiresAt",
        "expire_at",
        "expireAt",
        "end_time",
        "endTime",
    ];
    const ACCOUNT_KEYS: &[&str] = &[
        "chatgpt_subscription_active_until",
        "subscription_until",
        "subscriptionUntil",
        "expires_at",
        "expiresAt",
        "expire_at",
        "expireAt",
        "expired_at",
        "expiredAt",
        "renewal_at",
        "renewalAt",
        "renew_at",
        "renewAt",
        "end_time",
        "endTime",
        "valid_until",
        "validUntil",
    ];

    item.get("id_token")
        .and_then(|token| date_field(token, TOKEN_KEYS))
        .or_else(|| date_field(item, ACCOUNT_KEYS))
}

fn date_field(value: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(parse_datetime_value))
}

fn parse_datetime_value(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(raw) = value.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
            return Some(dt.with_timezone(&Utc));
        }
        if let Ok(timestamp) = raw.parse::<i64>() {
            return timestamp_datetime(timestamp);
        }
    }
    value.as_i64().and_then(timestamp_datetime)
}

fn timestamp_datetime(value: i64) -> Option<DateTime<Utc>> {
    if value <= 0 {
        return None;
    }
    if value > 10_000_000_000 {
        DateTime::from_timestamp_millis(value)
    } else {
        DateTime::from_timestamp(value, 0)
    }
}

fn normalize_percent(value: f64) -> f64 {
    if value <= 1.0 {
        value * 100.0
    } else {
        value
    }
}

fn stable_id(connection_id: &str, external_id: &str) -> String {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("{}:{}", connection_id, external_id).as_bytes(),
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::map_auth_files;

    #[test]
    fn maps_missing_fields() {
        let accounts = map_auth_files("c", &json!({ "data": [{ "email": "name@example.com" }] }));
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].plan_name, "Codex Plus");
    }

    #[test]
    fn filters_disabled_accounts() {
        let accounts = map_auth_files(
            "c",
            &json!({
                "data": [
                    { "email": "enabled@example.com", "enabled": true },
                    { "email": "disabled-flag@example.com", "disabled": true },
                    { "email": "disabled-enabled@example.com", "enabled": false },
                    { "email": "disabled-status@example.com", "status": "disabled" },
                    { "email": "disabled-state@example.com", "state": "disabled" }
                ]
            }),
        );

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].external_id, "enabled@example.com");
    }

    #[test]
    fn maps_subscription_until_aliases_and_timestamps() {
        let accounts = map_auth_files(
            "c",
            &json!({
                "data": [
                    { "email": "one@example.com", "subscriptionUntil": "2026-08-29T15:59:59Z" },
                    { "email": "two@example.com", "endTime": 1788019199000_i64 }
                ]
            }),
        );

        assert_eq!(accounts.len(), 2);
        assert_eq!(
            accounts[0].subscription_until.unwrap().to_rfc3339(),
            "2026-08-29T15:59:59+00:00"
        );
        assert_eq!(
            accounts[1].subscription_until.unwrap().to_rfc3339(),
            "2026-08-29T15:59:59+00:00"
        );
    }
}
