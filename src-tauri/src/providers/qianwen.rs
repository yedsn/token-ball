use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    quota::{AccountStatus, PeriodType, QuotaAccount, QuotaUnit, QuotaWindow},
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QianwenConfig {
    #[serde(default = "default_product_code")]
    pub qianwen_product_code: String,
    #[serde(default = "default_gateway_base_url")]
    pub qianwen_gateway_base_url: String,
    #[serde(default)]
    pub qianwen_cookie: Option<String>,
}

#[derive(Clone)]
pub struct QianwenClient {
    client: Client,
    base_url: Url,
    config: QianwenConfig,
}

impl QianwenClient {
    pub fn new(base_url: &str, raw_config: &str) -> AppResult<Self> {
        let mut config: QianwenConfig = serde_json::from_str(raw_config).map_err(|_| {
            AppError::Message("千问配置格式错误，请填写控制台 Cookie。".to_string())
        })?;
        config.qianwen_product_code = config.qianwen_product_code.trim().to_string();
        config.qianwen_gateway_base_url = config.qianwen_gateway_base_url.trim().to_string();
        config.qianwen_cookie = trim_optional(config.qianwen_cookie);
        if config.qianwen_product_code.is_empty() {
            config.qianwen_product_code = default_product_code();
        }
        if config.qianwen_gateway_base_url.is_empty() {
            config.qianwen_gateway_base_url = default_gateway_base_url();
        }
        if config.qianwen_cookie.is_none() {
            return Err(AppError::Message("千问官方控制台 API 需要填写登录态 Cookie".to_string()));
        }
        let base_url = if base_url.trim().is_empty() {
            Url::parse(&config.qianwen_gateway_base_url)?
        } else {
            Url::parse(base_url)?
        };
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()?,
            base_url,
            config,
        })
    }

    pub async fn test_connection(&self) -> AppResult<()> {
        self.get_personal_usage().await.map(|_| ())
    }

    pub async fn account_snapshot(&self, connection_id: &str) -> AppResult<Vec<QuotaAccount>> {
        let usage = self.get_personal_usage().await?;
        let summary = self.get_seat_subscription_summary().await.ok();
        let detail = self.get_subscription_detail().await.ok();
        Ok(build_accounts(
            connection_id,
            &self.config.qianwen_product_code,
            &usage,
            summary.as_ref(),
            detail.as_ref(),
        ))
    }

    async fn get_personal_usage(&self) -> AppResult<Value> {
        self.call_open_api(
            "sfm_bailian",
            "BroadScopeAspnGateway",
            serde_json::json!({
                "Api": "zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/usage",
                "Data": {
                    "cornerstoneParam": {
                        "domain": "platform.qianwenai.com",
                        "consoleSite": "QIANWENAI",
                        "console": "ONE_CONSOLE",
                        "xsp_lang": "zh-CN",
                        "protocol": "V2",
                        "productCode": "p_efm"
                    }
                },
                "V": "1.0"
            }),
        )
        .await
    }

    async fn get_seat_subscription_summary(&self) -> AppResult<Value> {
        self.call_open_api(
            "BssOpenAPI-V3",
            "GetSeatSubscriptionSummary",
            serde_json::json!({ "productCode": self.config.qianwen_product_code }),
        )
        .await
    }

    async fn get_subscription_detail(&self) -> AppResult<Value> {
        self.call_open_api(
            "BssOpenAPI-V3",
            "GetSubscriptionDetail",
            serde_json::json!({
                "productCode": self.config.qianwen_product_code,
                "pageNo": 1,
                "pageSize": 100
            }),
        )
        .await
    }

    async fn call_open_api(&self, product: &str, action: &str, params: Value) -> AppResult<Value> {
        let cookie = self
            .config
            .qianwen_cookie
            .as_deref()
            .ok_or_else(|| AppError::Message("千问控制台 Cookie 为空".to_string()))?;
        let sec_token = self.fetch_sec_token(cookie).await?;
        let url = self.api_url(product)?;
        let response = self
            .client
            .post(url)
            .query(&api_query(product, action, &params))
            .header("Accept", "application/json, text/plain, */*")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Origin", "https://platform.qianwenai.com")
            .header("Referer", "https://platform.qianwenai.com/home/billing/subscription/token-plan-individual")
            .header("Cookie", cookie)
            .form(&api_form(product, action, &sec_token, &params))
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::Message(format!(
                "千问官方控制台 API 返回 HTTP {}：{}",
                status, text
            )));
        }
        let value: Value = serde_json::from_str(&text).map_err(|error| {
            AppError::Message(format!(
                "千问官方控制台 API 响应不是有效 JSON：{}；响应前 500 字符：{}",
                error,
                text.chars().take(500).collect::<String>()
            ))
        })?;
        if is_success_response(&value) {
            Ok(value.get("data").cloned().unwrap_or(value))
        } else {
            Err(AppError::Message(format!(
                "千问官方控制台 API 调用失败：{}",
                error_message(&value)
            )))
        }
    }

    async fn fetch_sec_token(&self, cookie: &str) -> AppResult<String> {
        let url = Url::parse("https://platform-home.qianwenai.com/tool/user/info.json")?;
        let response = self
            .client
            .get(url)
            .header("Accept", "application/json, text/plain, */*")
            .header("Referer", "https://platform.qianwenai.com/home/billing/subscription/token-plan-individual")
            .header("Cookie", cookie)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::Message(format!(
                "千问 secToken 接口返回 HTTP {}：{}",
                status, text
            )));
        }
        let value: Value = serde_json::from_str(&text).map_err(|error| {
            AppError::Message(format!("千问 secToken 响应不是有效 JSON：{}", error))
        })?;
        value
            .pointer("/data/secToken")
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Message("千问 secToken 为空，请检查 Cookie 是否仍有效".to_string()))
    }

    fn api_url(&self, product: &str) -> AppResult<Url> {
        if product == "sfm_bailian" {
            Url::parse("https://cs-data.qianwenai.com/data/api.json").map_err(Into::into)
        } else {
            self.base_url.join("data/api.json").map_err(Into::into)
        }
    }
}

fn api_query(product: &str, action: &str, params: &Value) -> Vec<(String, String)> {
    let mut query = vec![
        ("product".to_string(), product.to_string()),
        ("action".to_string(), action.to_string()),
    ];
    if let Some(api) = params.get("Api").and_then(|value| value.as_str()) {
        query.push(("api".to_string(), api.to_string()));
    }
    query
}

fn api_form(product: &str, action: &str, sec_token: &str, params: &Value) -> Vec<(String, String)> {
    let mut form = vec![
        ("product".to_string(), product.to_string()),
        ("action".to_string(), action.to_string()),
        ("sec_token".to_string(), sec_token.to_string()),
        ("region".to_string(), "cn-beijing".to_string()),
        ("params".to_string(), params.to_string()),
    ];
    if let Some(api) = params.get("Api").and_then(|value| value.as_str()) {
        form.push(("api".to_string(), api.to_string()));
    }
    form
}

fn build_accounts(
    connection_id: &str,
    product_code: &str,
    usage: &Value,
    summary: Option<&Value>,
    detail: Option<&Value>,
) -> Vec<QuotaAccount> {
    let mut accounts = Vec::new();
    if let Some(account) = personal_usage_account(connection_id, product_code, usage) {
        accounts.push(account);
    }
    if let Some(data) = summary.and_then(|value| value.get("Data")) {
        let start_time = timestamp_millis(value_i64(data, &["StartTime"]));
        let end_time = timestamp_millis(value_i64(data, &["EndTime"]));
        for (index, group) in array_at(data, &["SubscriptionGroupList"]).iter().enumerate() {
            let spec_type = value_string(group, &["SpecType"]).unwrap_or_else(|| format!("seat-{index}"));
            let windows = build_group_windows(group, end_time);
            let critical_window_id = critical_window_id(&windows);
            accounts.push(QuotaAccount {
                id: format!("{connection_id}:qianwen-token-plan:{spec_type}"),
                connection_id: connection_id.to_string(),
                external_id: format!("qianwen-token-plan:{spec_type}"),
                display_name: format!("千问 Token Plan {}", plan_label(&spec_type)),
                masked_identifier: Some(product_code.to_string()),
                plan_name: format!(
                    "{} 席位 · {}",
                    value_f64(group, &["SubscriptionTotalNumber"]).unwrap_or(0.0),
                    product_code
                ),
                status: subscription_status(start_time, end_time),
                critical_window_id,
                next_reset_at: windows.iter().filter_map(|window| window.reset_at).min(),
                windows,
                success_count: None,
                failed_count: None,
                recent_requests: Vec::new(),
                subscription_until: end_time,
                chatgpt_account_id: None,
                synced_at: Utc::now(),
            });
        }
    }
    if accounts.is_empty() {
        accounts.push(placeholder_account(connection_id, product_code, "未返回 Token Plan 个人版用量"));
    }
    for (index, item) in addon_items(detail).iter().enumerate() {
        if let Some(account) = addon_account(connection_id, product_code, index, item) {
            accounts.push(account);
        }
    }
    accounts
}

fn personal_usage_account(
    connection_id: &str,
    product_code: &str,
    usage: &Value,
) -> Option<QuotaAccount> {
    let data = usage.pointer("/DataV2/data/data")?;
    let mut windows = Vec::new();
    if let Some(window) = percent_window(
        "qianwen-personal-5h",
        "5 小时",
        PeriodType::Rolling,
        Some(5 * 60 * 60),
        value_f64(data, &["per5HourPercentage"]),
        None,
    ) {
        windows.push(window);
    }
    if let Some(window) = percent_window_remaining(
        "qianwen-personal-weekly",
        "每周",
        PeriodType::Weekly,
        Some(7 * 24 * 60 * 60),
        value_f64(data, &["per1WeekPercentage"]),
        timestamp_millis(value_i64(data, &["per1WeekResetTime"])),
    ) {
        windows.push(window);
    }
    if windows.is_empty() {
        return None;
    }
    let critical_window_id = critical_window_id(&windows);
    Some(QuotaAccount {
        id: format!("{connection_id}:qianwen-token-plan-personal"),
        connection_id: connection_id.to_string(),
        external_id: "qianwen-token-plan-personal".to_string(),
        display_name: "千问 Token Plan 个人版".to_string(),
        masked_identifier: Some(product_code.to_string()),
        plan_name: product_code.to_string(),
        status: AccountStatus::Available,
        critical_window_id,
        next_reset_at: windows.iter().filter_map(|window| window.reset_at).min(),
        windows,
        success_count: None,
        failed_count: None,
        recent_requests: Vec::new(),
        subscription_until: None,
        chatgpt_account_id: None,
        synced_at: Utc::now(),
    })
}

fn percent_window(
    id: &str,
    name: &str,
    period_type: PeriodType,
    period_seconds: Option<i64>,
    used_percent: Option<f64>,
    reset_at: Option<DateTime<Utc>>,
) -> Option<QuotaWindow> {
    let used = used_percent?;
    let remaining = (100.0 - used).clamp(0.0, 100.0);
    Some(QuotaWindow {
        id: id.to_string(),
        name: name.to_string(),
        period_type,
        period_seconds,
        total: Some(100.0),
        used: Some(used.clamp(0.0, 100.0)),
        remaining: Some(remaining),
        remaining_percent: Some(remaining),
        unit: QuotaUnit::Percent,
        reset_at,
        is_active: true,
        is_current_constraint: true,
        data_source: "qianwen:sfm_bailian/BroadScopeAspnGateway/tokenplan-personal-usage".to_string(),
    })
}

fn percent_window_remaining(
    id: &str,
    name: &str,
    period_type: PeriodType,
    period_seconds: Option<i64>,
    remaining_percent: Option<f64>,
    reset_at: Option<DateTime<Utc>>,
) -> Option<QuotaWindow> {
    let remaining = remaining_percent?.clamp(0.0, 100.0);
    Some(QuotaWindow {
        id: id.to_string(),
        name: name.to_string(),
        period_type,
        period_seconds,
        total: Some(100.0),
        used: Some((100.0 - remaining).clamp(0.0, 100.0)),
        remaining: Some(remaining),
        remaining_percent: Some(remaining),
        unit: QuotaUnit::Percent,
        reset_at,
        is_active: true,
        is_current_constraint: true,
        data_source: "qianwen:sfm_bailian/BroadScopeAspnGateway/tokenplan-personal-usage".to_string(),
    })
}

fn build_group_windows(group: &Value, reset_at: Option<DateTime<Utc>>) -> Vec<QuotaWindow> {
    array_at(group, &["EquityList"])
        .iter()
        .enumerate()
        .filter_map(|(index, equity)| {
            let total = value_f64(equity, &["TotalValue"])?;
            let remaining = value_f64(equity, &["SurplusValue"])?;
            let used = (total - remaining).max(0.0);
            let percent = percent_remaining(total, remaining);
            Some(QuotaWindow {
                id: format!("qianwen-seat-{index}"),
                name: value_string(equity, &["Name", "EquityName"])
                    .unwrap_or_else(|| "Token Plan 主套餐".to_string()),
                period_type: PeriodType::Monthly,
                period_seconds: Some(30 * 24 * 60 * 60),
                total: Some(total),
                used: Some(used),
                remaining: Some(remaining.max(0.0)),
                remaining_percent: percent,
                unit: QuotaUnit::Credit,
                reset_at,
                is_active: true,
                is_current_constraint: true,
                data_source: "qianwen:BssOpenAPI-V3/GetSeatSubscriptionSummary".to_string(),
            })
        })
        .collect()
}

fn addon_account(
    connection_id: &str,
    product_code: &str,
    index: usize,
    item: &Value,
) -> Option<QuotaAccount> {
    let equity = array_at(item, &["EquityList"]).first().cloned()?;
    let total = value_f64(&equity, &["CycleTotalValue"])?;
    let remaining = value_f64(&equity, &["CycleSurplusValue"])?;
    let used = (total - remaining).max(0.0);
    let reset_at = timestamp_millis(value_i64(item, &["EndTime"]));
    let window = QuotaWindow {
        id: format!("qianwen-addon-{index}"),
        name: "Token Plan 加油包".to_string(),
        period_type: PeriodType::Custom,
        period_seconds: None,
        total: Some(total),
        used: Some(used),
        remaining: Some(remaining.max(0.0)),
        remaining_percent: percent_remaining(total, remaining),
        unit: QuotaUnit::Credit,
        reset_at,
        is_active: true,
        is_current_constraint: true,
        data_source: "qianwen:BssOpenAPI-V3/GetSubscriptionDetail".to_string(),
    };
    Some(QuotaAccount {
        id: format!("{connection_id}:qianwen-token-addon:{index}"),
        connection_id: connection_id.to_string(),
        external_id: format!("qianwen-token-addon:{index}"),
        display_name: value_string(item, &["InstanceName", "CommodityName"])
            .unwrap_or_else(|| "千问 Token Plan 加油包".to_string()),
        masked_identifier: value_string(item, &["InstanceCode"]),
        plan_name: product_code.to_string(),
        status: match value_string(item, &["Status"]).as_deref() {
            Some("NORMAL") | Some("valid") | Some("Valid") => AccountStatus::Available,
            Some("EXHAUST") | Some("exhaust") => AccountStatus::Exhausted,
            _ => AccountStatus::Unknown,
        },
        windows: vec![window],
        critical_window_id: Some(format!("qianwen-addon-{index}")),
        next_reset_at: reset_at,
        success_count: None,
        failed_count: None,
        recent_requests: Vec::new(),
        subscription_until: reset_at,
        chatgpt_account_id: None,
        synced_at: Utc::now(),
    })
}

fn addon_items(detail: Option<&Value>) -> Vec<Value> {
    detail
        .and_then(|value| value.get("Data"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
}

fn subscription_status(start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> AccountStatus {
    let now = Utc::now();
    if let Some(end) = end {
        if now > end {
            return AccountStatus::Exhausted;
        }
    }
    if let Some(start) = start {
        if now < start {
            return AccountStatus::Unknown;
        }
    }
    AccountStatus::Available
}

fn placeholder_account(connection_id: &str, product_code: &str, message: &str) -> QuotaAccount {
    QuotaAccount {
        id: format!("{connection_id}:qianwen-token-plan"),
        connection_id: connection_id.to_string(),
        external_id: "qianwen-token-plan".to_string(),
        display_name: "千问 Token Plan".to_string(),
        masked_identifier: Some(product_code.to_string()),
        plan_name: message.to_string(),
        status: AccountStatus::Unknown,
        windows: Vec::new(),
        critical_window_id: None,
        next_reset_at: None,
        success_count: None,
        failed_count: None,
        recent_requests: Vec::new(),
        subscription_until: None,
        chatgpt_account_id: None,
        synced_at: Utc::now(),
    }
}

fn critical_window_id(windows: &[QuotaWindow]) -> Option<String> {
    windows
        .iter()
        .filter_map(|window| window.remaining_percent.map(|percent| (window.id.clone(), percent)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| id)
}

fn value_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn value_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        let value = value.get(*key)?;
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|value| value.parse::<f64>().ok()))
    })
}

fn value_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        let value = value.get(*key)?;
        value
            .as_i64()
            .or_else(|| value.as_str().and_then(|value| value.parse::<i64>().ok()))
    })
}

fn array_at<'a>(value: &'a Value, keys: &[&str]) -> &'a Vec<Value> {
    static EMPTY: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(|value| value.as_array())
        .unwrap_or_else(|| EMPTY.get_or_init(Vec::new))
}

fn percent_remaining(total: f64, remaining: f64) -> Option<f64> {
    if total > 0.0 {
        Some((remaining / total * 100.0).clamp(0.0, 100.0))
    } else {
        None
    }
}

fn timestamp_millis(value: Option<i64>) -> Option<DateTime<Utc>> {
    let value = value?;
    if value <= 0 {
        return None;
    }
    if value > 10_000_000_000 {
        Utc.timestamp_millis_opt(value).single()
    } else {
        Utc.timestamp_opt(value, 0).single()
    }
}

fn is_success_response(value: &Value) -> bool {
    value
        .get("successResponse")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || value.get("code").and_then(|value| value.as_str()) == Some("200")
}

fn error_message(value: &Value) -> String {
    value
        .get("message")
        .or_else(|| value.get("msg"))
        .or_else(|| value.get("errorMsg"))
        .and_then(|value| value.as_str())
        .unwrap_or("未知错误")
        .to_string()
}

fn plan_label(value: &str) -> &str {
    match value {
        "standard" | "STANDARD" => "Standard",
        "pro" | "PRO" => "Pro",
        "max" | "MAX" => "Max",
        _ => value,
    }
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn default_product_code() -> String {
    "token-plan".to_string()
}

fn default_gateway_base_url() -> String {
    "https://platform-home.qianwenai.com".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_subscription_summary_to_quota_windows() {
        let raw = r#"
        {
          "Data": {
            "ProductCode": "token-plan",
            "StartTime": 1783094400000,
            "EndTime": 1785686400000,
            "SubscriptionGroupList": [
              {
                "SpecType": "standard",
                "SubscriptionTotalNumber": 2,
                "EquityList": [
                  { "SurplusValue": "1500", "TotalValue": "2000" }
                ]
              }
            ]
          }
        }
        "#;
        let usage: Value = serde_json::from_str(r#"{ "DataV2": { "data": { "data": {} } } }"#).unwrap();
        let summary: Value = serde_json::from_str(raw).unwrap();
        let accounts = build_accounts("conn", "token-plan", &usage, Some(&summary), None);
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].external_id, "qianwen-token-plan:standard");
        assert_eq!(accounts[0].windows[0].remaining, Some(1500.0));
        assert_eq!(accounts[0].windows[0].used, Some(500.0));
        assert_eq!(accounts[0].windows[0].remaining_percent, Some(75.0));
    }

    #[test]
    fn maps_subscription_detail_addons() {
        let usage: Value = serde_json::from_str(r#"{ "DataV2": { "data": { "data": {} } } }"#).unwrap();
        let summary: Value = serde_json::from_str(r#"{ "Data": { "SubscriptionGroupList": [] } }"#).unwrap();
        let detail: Value = serde_json::from_str(
            r#"
            {
              "Data": [
                {
                  "InstanceCode": "addon-1",
                  "InstanceName": "加油包 A",
                  "Status": "NORMAL",
                  "EndTime": 1785686400000,
                  "EquityList": [
                    { "CycleSurplusValue": "300", "CycleTotalValue": "1000" }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap();
        let accounts = build_accounts("conn", "token-plan", &usage, Some(&summary), Some(&detail));
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[1].display_name, "加油包 A");
        assert_eq!(accounts[1].windows[0].remaining_percent, Some(30.0));
    }

    #[test]
    fn maps_personal_usage_percent_windows() {
        let usage: Value = serde_json::from_str(
            r#"
            {
              "DataV2": {
                "data": {
                  "data": {
                    "per5HourPercentage": 0.0,
                    "per1WeekPercentage": 1.0,
                    "per1WeekResetTime": 1785067800000
                  }
                }
              }
            }
            "#,
        )
        .unwrap();
        let accounts = build_accounts("conn", "token-plan", &usage, None, None);
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].external_id, "qianwen-token-plan-personal");
        assert_eq!(accounts[0].windows.len(), 2);
        assert_eq!(accounts[0].windows[0].remaining_percent, Some(100.0));
        assert_eq!(accounts[0].windows[1].remaining_percent, Some(1.0));
        assert!(accounts[0].windows[1].reset_at.is_some());
    }
}
