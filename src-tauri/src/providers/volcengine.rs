use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use crate::{
    error::{AppError, AppResult},
    quota::{AccountStatus, PeriodType, QuotaAccount, QuotaUnit, QuotaWindow},
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolcengineConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_service")]
    pub service: String,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default = "default_true")]
    pub sync_agent_plan: bool,
    #[serde(default = "default_true")]
    pub sync_coding_plan: bool,
    #[serde(default)]
    pub coding_project_name: Option<String>,
    #[serde(default)]
    pub coding_seat_id: Option<String>,
    #[serde(default)]
    pub coding_web_base_url: Option<String>,
    #[serde(default)]
    pub coding_web_cookie: Option<String>,
}

#[derive(Clone)]
pub struct VolcengineClient {
    client: Client,
    base_url: Url,
    config: VolcengineConfig,
}

impl VolcengineClient {
    pub fn new(base_url: &str, raw_config: &str) -> AppResult<Self> {
        let base_url = Url::parse(base_url)?;
        let mut config: VolcengineConfig = serde_json::from_str(raw_config).map_err(|_| {
            AppError::Message("火山引擎配置格式错误，请填写 Access Key ID / Secret Access Key / Region / Service。".to_string())
        })?;
        config.access_key_id = config.access_key_id.trim().to_string();
        config.secret_access_key = config.secret_access_key.trim().to_string();
        config.region = config.region.trim().to_string();
        config.service = config.service.trim().to_string();
        config.channel = config.channel.trim().to_string();
        config.coding_project_name = trim_optional(config.coding_project_name);
        config.coding_seat_id = trim_optional(config.coding_seat_id);
        config.coding_web_base_url = trim_optional(config.coding_web_base_url);
        config.coding_web_cookie = trim_optional(config.coding_web_cookie);
        if !config.channel.eq_ignore_ascii_case("web")
            && (config.access_key_id.trim().is_empty()
                || config.secret_access_key.trim().is_empty())
        {
            return Err(AppError::Message(
                "火山引擎 Access Key ID 和 Secret Access Key 不能为空".to_string(),
            ));
        }
        if config.channel.eq_ignore_ascii_case("web") && config.coding_web_cookie.is_none() {
            return Err(AppError::Message(
                "火山引擎页面渠道需要填写控制台 Cookie".to_string(),
            ));
        }
        if config.region.is_empty() {
            config.region = default_region();
        }
        if config.service.is_empty() {
            config.service = default_service();
        }
        if config.channel.is_empty() {
            config.channel = default_channel();
        }
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()?,
            base_url,
            config,
        })
    }

    pub async fn test_connection(&self) -> AppResult<()> {
        if self.is_web_channel() {
            self.get_coding_plan_usage_from_web().await.map(|_| ())
        } else {
            self.get_afp_usage().await.map(|_| ())
        }
    }

    pub async fn account_snapshot(&self, connection_id: &str) -> AppResult<Vec<QuotaAccount>> {
        let mut accounts = Vec::new();
        if self.config.sync_agent_plan && !self.is_web_channel() {
            let usage = self.get_afp_usage().await?;
            let plan = self.get_personal_plan("AgentPlan").await.ok();
            let plan_type = usage
                .result
                .plan_type
                .as_ref()
                .cloned()
                .or_else(|| plan.as_ref().and_then(|plan| plan.result.plan_type.clone()))
                .unwrap_or_else(|| "Agent Plan".to_string());
            let windows = build_windows(&usage.result);
            let subscription_until = plan
                .as_ref()
                .and_then(|plan| plan.result.end_time.as_deref())
                .and_then(parse_rfc3339);
            let status = match plan.as_ref().and_then(|plan| plan.result.status.as_deref()) {
                Some("Running") | None => AccountStatus::Available,
                Some("Expired") => AccountStatus::Exhausted,
                _ => AccountStatus::Unknown,
            };
            accounts.push(QuotaAccount {
                id: format!("{connection_id}:volcengine-afp"),
                connection_id: connection_id.to_string(),
                external_id: "volcengine-afp".to_string(),
                display_name: "Agent Plan AFP".to_string(),
                masked_identifier: Some(mask_ak(&self.config.access_key_id)),
                plan_name: plan_type,
                status,
                critical_window_id: critical_window_id(&windows),
                next_reset_at: windows.iter().filter_map(|window| window.reset_at).min(),
                windows,
                success_count: None,
                failed_count: None,
                recent_requests: Vec::new(),
                subscription_until,
                chatgpt_account_id: None,
                synced_at: Utc::now(),
            });
        } else if self.config.sync_agent_plan && self.is_web_channel() {
            let subscription = self.get_agent_subscribe_trade_from_web().await.ok();
            accounts.push(agent_placeholder_account(
                connection_id,
                "页面渠道暂未接入 Agent Plan 接口".to_string(),
                subscription
                    .as_ref()
                    .and_then(subscribe_trade_subscription_until),
            ));
        }

        if !self.config.sync_coding_plan {
            return Ok(accounts);
        }

        if self.is_web_channel() {
            accounts.push(self.web_coding_plan_account(connection_id).await?);
        } else if self.config.coding_seat_id.is_none() {
            accounts.push(self.personal_coding_plan_account(connection_id).await?);
        } else if let Some(seat_usages) = self.list_seat_info_usages().await? {
            if seat_usages.is_empty() {
                accounts.push(coding_placeholder_account(
                    connection_id,
                    "未返回席位用量".to_string(),
                ));
            }
            for (index, seat_usage) in seat_usages.into_iter().enumerate() {
                let windows = build_coding_windows(&seat_usage);
                let seat_label = seat_usage
                    .seat_id
                    .clone()
                    .unwrap_or_else(|| format!("seat-{index}"));
                accounts.push(QuotaAccount {
                    id: format!("{connection_id}:volcengine-coding-plan:{seat_label}"),
                    connection_id: connection_id.to_string(),
                    external_id: format!("volcengine-coding-plan:{seat_label}"),
                    display_name: "Coding Plan".to_string(),
                    masked_identifier: seat_usage.seat_id.clone(),
                    plan_name: seat_usage
                        .project_name
                        .clone()
                        .unwrap_or_else(|| "ListSeatInfoUsages".to_string()),
                    status: AccountStatus::Available,
                    windows,
                    critical_window_id: None,
                    next_reset_at: None,
                    success_count: None,
                    failed_count: None,
                    recent_requests: Vec::new(),
                    subscription_until: None,
                    chatgpt_account_id: None,
                    synced_at: Utc::now(),
                });
            }
        } else {
            accounts.push(coding_placeholder_account(
                connection_id,
                "未返回席位信息".to_string(),
            ));
        }

        Ok(accounts)
    }

    fn is_web_channel(&self) -> bool {
        self.config.channel.eq_ignore_ascii_case("web")
    }

    async fn personal_coding_plan_account(&self, connection_id: &str) -> AppResult<QuotaAccount> {
        let plan = self.get_personal_plan("CodingPlan").await?;
        let hourly_details = self.get_usage_details("Hour", 1).await?;
        let daily_details = self.get_usage_details("Day", 30).await?;
        let windows = build_personal_coding_windows(
            &hourly_details.result.details,
            &daily_details.result.details,
            Utc::now(),
        );
        let subscription_until = plan
            .result
            .end_time
            .and_then(|value| parse_rfc3339(&value))
            .or_else(|| usage_details_subscription_until(&hourly_details.result))
            .or_else(|| usage_details_subscription_until(&daily_details.result));
        let status = match plan.result.status.as_deref() {
            Some("Running") => AccountStatus::Available,
            Some("Expired") => AccountStatus::Exhausted,
            _ => AccountStatus::Unknown,
        };
        let critical_window_id = critical_window_id(&windows);
        let next_reset_at = windows.iter().filter_map(|window| window.reset_at).min();
        Ok(QuotaAccount {
            id: format!("{connection_id}:volcengine-coding-plan"),
            connection_id: connection_id.to_string(),
            external_id: "volcengine-coding-plan".to_string(),
            display_name: "Coding Plan 个人版".to_string(),
            masked_identifier: Some(mask_ak(&self.config.access_key_id)),
            plan_name: format!(
                "{} · {}",
                plan.result
                    .plan_type
                    .clone()
                    .unwrap_or_else(|| "CodingPlan".to_string()),
                plan.result
                    .status
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string())
            ),
            status,
            windows,
            critical_window_id,
            next_reset_at,
            success_count: None,
            failed_count: None,
            recent_requests: Vec::new(),
            subscription_until,
            chatgpt_account_id: None,
            synced_at: Utc::now(),
        })
    }

    async fn web_coding_plan_account(&self, connection_id: &str) -> AppResult<QuotaAccount> {
        let usage = self.get_coding_plan_usage_from_web().await?;
        let subscription = self.get_subscribe_trade_from_web().await.ok();
        let windows = usage
            .result
            .as_ref()
            .map(|result| build_web_coding_windows(result))
            .unwrap_or_default();
        let critical_window_id = critical_window_id(&windows);
        let next_reset_at = windows.iter().filter_map(|window| window.reset_at).min();
        let subscription_until = subscription
            .as_ref()
            .and_then(subscribe_trade_subscription_until)
            .or_else(|| {
                usage
                    .result
                    .as_ref()
                    .and_then(web_coding_subscription_until)
            });
        Ok(QuotaAccount {
            id: format!("{connection_id}:volcengine-coding-plan-web"),
            connection_id: connection_id.to_string(),
            external_id: "volcengine-coding-plan-web".to_string(),
            display_name: "Coding Plan 个人版".to_string(),
            masked_identifier: Some("控制台 Cookie".to_string()),
            plan_name: "页面渠道".to_string(),
            status: AccountStatus::Available,
            windows,
            critical_window_id,
            next_reset_at,
            success_count: None,
            failed_count: None,
            recent_requests: Vec::new(),
            subscription_until,
            chatgpt_account_id: None,
            synced_at: Utc::now(),
        })
    }

    async fn get_afp_usage(&self) -> AppResult<GetAfpUsageResponse> {
        self.openapi_get(&[
            ("Action", "GetAFPUsage".to_string()),
            ("Version", "2024-01-01".to_string()),
        ])
        .await
    }

    async fn get_personal_plan(&self, plan: &str) -> AppResult<GetPersonalPlanResponse> {
        let params = vec![
            ("Action", "GetPersonalPlan".to_string()),
            ("Version", "2024-01-01".to_string()),
        ];
        let body = serde_json::json!({ "Plan": plan }).to_string();
        self.openapi_post(&params, &body).await
    }

    async fn get_usage_details(
        &self,
        query_interval: &str,
        days: i64,
    ) -> AppResult<GetUsageDetailsResponse> {
        let params = vec![
            ("Action", "GetUsageDetails".to_string()),
            ("Version", "2024-01-01".to_string()),
        ];
        let end = Utc::now().date_naive();
        let start = end - chrono::Duration::days(days);
        let body = serde_json::json!({
            "QueryInterval": query_interval,
            "Filter": {
                "StartTime": start.format("%Y-%m-%d").to_string(),
                "EndTime": end.format("%Y-%m-%d").to_string()
            }
        })
        .to_string();
        self.openapi_post(&params, &body).await
    }

    async fn get_coding_plan_usage_from_web(&self) -> AppResult<WebCodingPlanUsageResponse> {
        let cookie = self
            .config
            .coding_web_cookie
            .as_deref()
            .ok_or_else(|| AppError::Message("火山控制台 Cookie 为空".to_string()))?;
        let base_url = self
            .config
            .coding_web_base_url
            .as_deref()
            .unwrap_or("https://console.volcengine.com/api/top");
        let region = self.config.region.trim();
        let service = self.config.service.trim();
        let url = build_web_coding_usage_url(base_url, service, region)?;
        let project_name = self.config.coding_project_name.as_deref();
        let body = if let Some(project_name) = project_name {
            serde_json::json!({ "ProjectName": project_name }).to_string()
        } else {
            "{}".to_string()
        };
        let csrf_token = extract_cookie_value(cookie, "csrfToken").unwrap_or_default();
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Accept", "application/json, text/plain, */*")
            .header("Origin", "https://console.volcengine.com")
            .header(
                "Referer",
                "https://console.volcengine.com/ark/region:cn-beijing/model-settings/coding-plan",
            )
            .header("Cookie", cookie)
            .header("x-csrf-token", csrf_token)
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::Message(format!(
                "火山控制台 Coding Plan 接口返回 HTTP {}：{}",
                status, text
            )));
        }
        serde_json::from_str(&text).map_err(|error| {
            AppError::Message(format!(
                "火山控制台 Coding Plan 响应不是有效 JSON：{}；响应前 500 字符：{}",
                error,
                text.chars().take(500).collect::<String>()
            ))
        })
    }

    async fn get_subscribe_trade_from_web(&self) -> AppResult<Value> {
        let cookie = self
            .config
            .coding_web_cookie
            .as_deref()
            .ok_or_else(|| AppError::Message("火山控制台 Cookie 为空".to_string()))?;
        let base_url = self
            .config
            .coding_web_base_url
            .as_deref()
            .unwrap_or("https://console.volcengine.com/api/top");
        let region = self.config.region.trim();
        let service = self.config.service.trim();
        let url = build_web_console_action_url(base_url, service, region, "ListSubscribeTrade")?;
        let body = serde_json::json!({
            "ResourceTypes": ["CodingPlan"],
            "ResourceNames": [""],
            "BizInfos": ["lite"]
        })
        .to_string();
        let csrf_token = extract_cookie_value(cookie, "csrfToken").unwrap_or_default();
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Accept", "application/json, text/plain, */*")
            .header("Origin", "https://console.volcengine.com")
            .header(
                "Referer",
                "https://console.volcengine.com/ark/region:cn-beijing/model-settings/coding-plan",
            )
            .header("Cookie", cookie)
            .header("x-csrf-token", csrf_token)
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::Message(format!(
                "火山控制台 ListSubscribeTrade 接口返回 HTTP {}：{}",
                status, text
            )));
        }
        serde_json::from_str(&text).map_err(|error| {
            AppError::Message(format!(
                "火山控制台 ListSubscribeTrade 响应不是有效 JSON：{}；响应前 500 字符：{}",
                error,
                text.chars().take(500).collect::<String>()
            ))
        })
    }

    async fn get_agent_subscribe_trade_from_web(&self) -> AppResult<Value> {
        let cookie = self
            .config
            .coding_web_cookie
            .as_deref()
            .ok_or_else(|| AppError::Message("火山控制台 Cookie 为空".to_string()))?;
        let base_url = self
            .config
            .coding_web_base_url
            .as_deref()
            .unwrap_or("https://console.volcengine.com/api/top");
        let region = self.config.region.trim();
        let service = self.config.service.trim();
        let url = build_web_console_action_url(base_url, service, region, "ListSubscribeTrade")?;
        let body = serde_json::json!({
            "ResourceTypes": ["AgentPlan"],
            "ResourceNames": ["RealAgentPlanPersonal"],
            "BizInfos": ["small", "medium", "large", "xlarge"]
        })
        .to_string();
        let csrf_token = extract_cookie_value(cookie, "csrfToken").unwrap_or_default();
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Accept", "application/json, text/plain, */*")
            .header("Origin", "https://console.volcengine.com")
            .header(
                "Referer",
                "https://console.volcengine.com/ark/region:cn-beijing/subscription/agent-plan",
            )
            .header("Cookie", cookie)
            .header("x-csrf-token", csrf_token)
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::Message(format!(
                "火山控制台 Agent Plan 订阅接口返回 HTTP {}：{}",
                status, text
            )));
        }
        serde_json::from_str(&text).map_err(|error| {
            AppError::Message(format!(
                "火山控制台 Agent Plan 订阅响应不是有效 JSON：{}；响应前 500 字符：{}",
                error,
                text.chars().take(500).collect::<String>()
            ))
        })
    }

    async fn list_seat_info_usages(&self) -> AppResult<Option<Vec<SeatInfoUsage>>> {
        let Some(project_name) = self.config.coding_project_name.as_ref() else {
            return Ok(None);
        };
        let seat_ids =
            if let Some(seat_ids) = coding_seat_ids(self.config.coding_seat_id.as_deref()) {
                seat_ids
            } else {
                self.list_coding_seat_ids(project_name).await?
            };
        if seat_ids.is_empty() {
            return Ok(None);
        }
        let params = vec![
            ("Action", "ListSeatInfoUsages".to_string()),
            ("Version", "2024-01-01".to_string()),
        ];
        let body =
            serde_json::json!({ "ProjectName": project_name, "SeatIDs": seat_ids }).to_string();
        let response: ListSeatInfoUsagesResponse = self.openapi_post(&params, &body).await?;
        let seats = extract_seat_info_usages(&response.result);
        Ok(Some(seats))
    }

    async fn list_coding_seat_ids(&self, project_name: &str) -> AppResult<Vec<String>> {
        let params = vec![
            ("Action", "ListSeatInfos".to_string()),
            ("Version", "2024-01-01".to_string()),
        ];
        let body = serde_json::json!({
            "Filter": {},
            "ProjectName": project_name,
            "PageNum": 1,
            "PageSize": 100
        })
        .to_string();
        let response: ListSeatInfosResponse = self.openapi_post(&params, &body).await?;
        Ok(extract_seat_infos(&response.result)
            .into_iter()
            .filter_map(|seat| seat.seat_id)
            .collect())
    }

    async fn openapi_get<T>(&self, params: &[(impl AsRef<str>, String)]) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.openapi_request("GET", params, "", None).await
    }

    async fn openapi_post<T>(
        &self,
        params: &[(impl AsRef<str>, String)],
        body: &str,
    ) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.openapi_request(
            "POST",
            params,
            body,
            Some("application/json; charset=utf-8"),
        )
        .await
    }

    async fn openapi_request<T>(
        &self,
        method: &str,
        params: &[(impl AsRef<str>, String)],
        body: &str,
        content_type: Option<&str>,
    ) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let date = Utc::now();
        let x_date = date.format("%Y%m%dT%H%M%SZ").to_string();
        let short_date = date.format("%Y%m%d").to_string();
        let content_sha256 = sha256_hex(body.as_bytes());
        let host = self
            .base_url
            .host_str()
            .ok_or_else(|| AppError::Message("火山引擎 Host 无效".to_string()))?;
        let path = if self.base_url.path().is_empty() {
            "/"
        } else {
            self.base_url.path()
        };
        let query = canonical_query(params);
        let signed_headers = "host;x-date";
        let canonical_headers = format!("host:{host}\nx-date:{x_date}\n");
        let canonical_request = format!(
            "{method}\n{path}\n{query}\n{canonical_headers}\n{signed_headers}\n{content_sha256}"
        );
        let credential_scope = format!(
            "{}/{}/{}/request",
            short_date, self.config.region, self.config.service
        );
        let string_to_sign = format!(
            "HMAC-SHA256\n{x_date}\n{credential_scope}\n{}",
            sha256_hex(canonical_request.as_bytes())
        );
        let signing_key = signing_key(
            &self.config.secret_access_key,
            &short_date,
            &self.config.region,
            &self.config.service,
        )?;
        let signature = hmac_hex(&signing_key, string_to_sign.as_bytes())?;
        let authorization = format!(
            "HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.config.access_key_id
        );
        let url = self.base_url.join(&format!("?{query}"))?;
        let method = Method::from_bytes(method.as_bytes())
            .map_err(|error| AppError::Message(format!("火山引擎请求方法无效：{error}")))?;
        let mut request = self
            .client
            .request(method, url)
            .header("Host", host)
            .header("X-Date", x_date)
            .header("Authorization", authorization);
        if let Some(content_type) = content_type {
            request = request
                .header("Content-Type", content_type)
                .body(body.to_string());
        }
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                return Err(AppError::Message(format!(
                    "火山引擎 OpenAPI 认证失败，接口返回 HTTP {}：{}",
                    status, text
                )));
            }
            return Err(AppError::Message(format!(
                "火山引擎 OpenAPI 返回 HTTP {}：{}",
                status, text
            )));
        }
        serde_json::from_str(&text).map_err(|error| AppError::Message(error.to_string()))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GetAfpUsageResponse {
    result: AfpUsageResult,
}

#[derive(Debug, Deserialize)]
struct AfpUsageResult {
    #[serde(rename = "PlanType")]
    plan_type: Option<String>,
    #[serde(rename = "AFPFiveHour")]
    afp_five_hour: Option<AfpWindow>,
    #[serde(rename = "AFPDaily")]
    afp_daily: Option<AfpWindow>,
    #[serde(rename = "AFPWeekly")]
    afp_weekly: Option<AfpWindow>,
    #[serde(rename = "AFPMonthly")]
    afp_monthly: Option<AfpWindow>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AfpWindow {
    quota: f64,
    used: f64,
    reset_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GetPersonalPlanResponse {
    result: PersonalPlanResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PersonalPlanResult {
    plan_type: Option<String>,
    status: Option<String>,
    end_time: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GetUsageDetailsResponse {
    result: UsageDetailsResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UsageDetailsResult {
    #[serde(default)]
    details: Vec<UsageDetailItem>,
    end_time: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UsageDetailItem {
    time: i64,
    usage: f64,
    unit: Option<String>,
    billing_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListSeatInfoUsagesResponse {
    result: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListSeatInfosResponse {
    result: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatInfo {
    #[serde(rename = "SeatID")]
    seat_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatInfoUsage {
    #[serde(rename = "SeatID")]
    seat_id: Option<String>,
    project_name: Option<String>,
    short_term_usage: Option<f64>,
    weekly_usage: Option<f64>,
    monthly_usage: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WebCodingPlanUsageResponse {
    #[serde(default)]
    result: Option<WebCodingPlanUsageResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WebCodingPlanUsageResult {
    #[serde(default)]
    quota_usage: Vec<WebCodingQuotaUsage>,
    update_timestamp: Option<i64>,
    end_time: Option<Value>,
    expire_time: Option<Value>,
    expired_time: Option<Value>,
    subscription_end_time: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WebCodingQuotaUsage {
    level: String,
    percent: Option<f64>,
    reset_timestamp: Option<i64>,
}

fn build_windows(result: &AfpUsageResult) -> Vec<QuotaWindow> {
    [
        (
            "afp-five-hour",
            "5 小时 AFP",
            PeriodType::Custom,
            result.afp_five_hour.as_ref(),
        ),
        (
            "afp-daily",
            "每日 AFP",
            PeriodType::Daily,
            result.afp_daily.as_ref(),
        ),
        (
            "afp-weekly",
            "每周 AFP",
            PeriodType::Weekly,
            result.afp_weekly.as_ref(),
        ),
        (
            "afp-monthly",
            "每月 AFP",
            PeriodType::Monthly,
            result.afp_monthly.as_ref(),
        ),
    ]
    .into_iter()
    .filter_map(|(id, name, period_type, window)| {
        window.map(|window| quota_window(id, name, period_type, window))
    })
    .collect()
}

fn quota_window(id: &str, name: &str, period_type: PeriodType, window: &AfpWindow) -> QuotaWindow {
    let remaining = (window.quota - window.used).max(0.0);
    let remaining_percent = if window.quota > 0.0 {
        Some((remaining / window.quota * 100.0).clamp(0.0, 100.0))
    } else {
        None
    };
    QuotaWindow {
        id: id.to_string(),
        name: name.to_string(),
        period_type,
        period_seconds: None,
        total: Some(window.quota),
        used: Some(window.used),
        remaining: Some(remaining),
        remaining_percent,
        unit: QuotaUnit::Afp,
        reset_at: window.reset_time.and_then(timestamp_millis),
        is_active: true,
        is_current_constraint: true,
        data_source: "volcengine:GetAFPUsage".to_string(),
    }
}

fn build_coding_windows(result: &SeatInfoUsage) -> Vec<QuotaWindow> {
    [
        (
            "coding-short-term",
            "短期用量",
            PeriodType::Custom,
            result.short_term_usage,
        ),
        (
            "coding-weekly",
            "每周用量",
            PeriodType::Weekly,
            result.weekly_usage,
        ),
        (
            "coding-monthly",
            "每月用量",
            PeriodType::Monthly,
            result.monthly_usage,
        ),
    ]
    .into_iter()
    .filter_map(|(id, name, period_type, used_percent)| {
        used_percent.map(|used_percent| {
            let remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0);
            QuotaWindow {
                id: id.to_string(),
                name: name.to_string(),
                period_type,
                period_seconds: None,
                total: Some(100.0),
                used: Some(used_percent),
                remaining: Some(remaining_percent),
                remaining_percent: Some(remaining_percent),
                unit: QuotaUnit::Percent,
                reset_at: None,
                is_active: true,
                is_current_constraint: true,
                data_source: "volcengine:ListSeatInfoUsages".to_string(),
            }
        })
    })
    .collect()
}

fn build_personal_coding_windows(
    hourly_details: &[UsageDetailItem],
    daily_details: &[UsageDetailItem],
    now: DateTime<Utc>,
) -> Vec<QuotaWindow> {
    let five_hour_tokens = sum_usage_since(
        hourly_details,
        now - chrono::Duration::hours(5),
        Some("WithinPlan"),
    );
    let weekly_tokens = sum_usage_since(
        daily_details,
        now - chrono::Duration::days(7),
        Some("WithinPlan"),
    );
    let monthly_tokens = sum_usage_since(
        daily_details,
        now - chrono::Duration::days(30),
        Some("WithinPlan"),
    );
    vec![
        usage_window(
            "coding-personal-5h",
            "近 5 小时",
            PeriodType::Custom,
            Some(5 * 60 * 60),
            five_hour_tokens,
        ),
        usage_window(
            "coding-personal-weekly",
            "近 7 天",
            PeriodType::Weekly,
            Some(7 * 24 * 60 * 60),
            weekly_tokens,
        ),
        usage_window(
            "coding-personal-monthly",
            "近 30 天",
            PeriodType::Monthly,
            Some(30 * 24 * 60 * 60),
            monthly_tokens,
        ),
    ]
}

fn build_web_coding_windows(result: &WebCodingPlanUsageResult) -> Vec<QuotaWindow> {
    let order = ["session", "weekly", "monthly"];
    let mut items: Vec<_> = result.quota_usage.iter().collect();
    items.sort_by_key(|item| {
        order
            .iter()
            .position(|level| *level == item.level)
            .unwrap_or(order.len())
    });
    items
        .into_iter()
        .filter_map(|item| web_coding_window(item, result.update_timestamp))
        .collect()
}

fn web_coding_subscription_until(result: &WebCodingPlanUsageResult) -> Option<DateTime<Utc>> {
    result
        .end_time
        .as_ref()
        .or(result.expire_time.as_ref())
        .or(result.expired_time.as_ref())
        .or(result.subscription_end_time.as_ref())
        .and_then(parse_datetime_value)
}

fn usage_details_subscription_until(result: &UsageDetailsResult) -> Option<DateTime<Utc>> {
    result.end_time.as_ref().and_then(parse_datetime_value)
}

fn subscribe_trade_subscription_until(value: &Value) -> Option<DateTime<Utc>> {
    let mut dates = Vec::new();
    if let Some(info_list) = value.get("InfoList") {
        collect_subscription_dates(info_list, &mut dates);
    }
    if dates.is_empty() {
        collect_subscription_dates(value, &mut dates);
    }
    dates.into_iter().min()
}

fn collect_subscription_dates(value: &Value, dates: &mut Vec<DateTime<Utc>>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_subscription_dates(item, dates);
            }
        }
        Value::Object(map) => {
            for key in [
                "EndTime",
                "endTime",
                "ExpireTime",
                "expireTime",
                "ExpiredTime",
                "expiredTime",
                "SubscriptionEndTime",
                "subscriptionEndTime",
                "ValidUntil",
                "validUntil",
            ] {
                if let Some(date) = map.get(key).and_then(parse_datetime_value) {
                    dates.push(date);
                }
            }
            for child in map.values() {
                collect_subscription_dates(child, dates);
            }
        }
        _ => {}
    }
}

fn web_coding_window(
    item: &WebCodingQuotaUsage,
    update_timestamp: Option<i64>,
) -> Option<QuotaWindow> {
    let used_percent = item.percent?;
    let remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0);
    let (name, period_type, period_seconds) = match item.level.as_str() {
        "session" => ("当前会话", PeriodType::Custom, None),
        "weekly" => ("近 1 周", PeriodType::Weekly, Some(7 * 24 * 60 * 60)),
        "monthly" => ("近 1 月", PeriodType::Monthly, Some(30 * 24 * 60 * 60)),
        value => (value, PeriodType::Unknown, None),
    };
    Some(QuotaWindow {
        id: format!("coding-web-{}", item.level),
        name: name.to_string(),
        period_type,
        period_seconds,
        total: Some(100.0),
        used: Some(used_percent),
        remaining: Some(remaining_percent),
        remaining_percent: Some(remaining_percent),
        unit: QuotaUnit::Percent,
        reset_at: item
            .reset_timestamp
            .or(update_timestamp)
            .and_then(timestamp_seconds_or_millis),
        is_active: true,
        is_current_constraint: true,
        data_source: "volcengine-web:GetCodingPlanUsage".to_string(),
    })
}

fn usage_window(
    id: &str,
    name: &str,
    period_type: PeriodType,
    period_seconds: Option<i64>,
    used: f64,
) -> QuotaWindow {
    QuotaWindow {
        id: id.to_string(),
        name: name.to_string(),
        period_type,
        period_seconds,
        total: None,
        used: Some(used),
        remaining: None,
        remaining_percent: None,
        unit: QuotaUnit::Token,
        reset_at: None,
        is_active: true,
        is_current_constraint: true,
        data_source: "volcengine:GetUsageDetails".to_string(),
    }
}

fn sum_usage_since(
    details: &[UsageDetailItem],
    start_at: DateTime<Utc>,
    billing_type: Option<&str>,
) -> f64 {
    details
        .iter()
        .filter(|item| item.unit.as_deref().unwrap_or("Tokens") == "Tokens")
        .filter(|item| billing_type.is_none() || item.billing_type.as_deref() == billing_type)
        .filter(|item| {
            timestamp_millis(item.time)
                .map(|time| time >= start_at)
                .unwrap_or(false)
        })
        .map(|item| item.usage)
        .sum()
}

fn extract_seat_infos(result: &Value) -> Vec<SeatInfo> {
    let mut seats = Vec::new();
    collect_seat_infos(result, &mut seats);
    seats
}

fn collect_seat_infos(value: &Value, seats: &mut Vec<SeatInfo>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_seat_infos(item, seats);
            }
        }
        Value::Object(map) => {
            if value.get("SeatID").is_some() && !has_usage_fields(value) {
                if let Ok(seat) = serde_json::from_value::<SeatInfo>(value.clone()) {
                    seats.push(seat);
                    return;
                }
            }
            for key in ["Data", "Items", "List", "SeatInfos"] {
                if let Some(child) = map.get(key) {
                    collect_seat_infos(child, seats);
                }
            }
        }
        _ => {}
    }
}

fn extract_seat_info_usages(result: &Value) -> Vec<SeatInfoUsage> {
    let mut usages = Vec::new();
    collect_seat_info_usages(result, &mut usages);
    usages
}

fn collect_seat_info_usages(value: &Value, usages: &mut Vec<SeatInfoUsage>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_seat_info_usages(item, usages);
            }
        }
        Value::Object(map) => {
            if has_usage_fields(value) {
                if let Ok(usage) = serde_json::from_value::<SeatInfoUsage>(value.clone()) {
                    usages.push(usage);
                    return;
                }
            }
            for key in ["SeatInfoUsages", "SeatInfoUsage", "Items", "List", "Data"] {
                if let Some(child) = map.get(key) {
                    collect_seat_info_usages(child, usages);
                }
            }
        }
        _ => {}
    }
}

fn has_usage_fields(value: &Value) -> bool {
    value.get("ShortTermUsage").is_some()
        || value.get("WeeklyUsage").is_some()
        || value.get("MonthlyUsage").is_some()
}

fn coding_placeholder_account(connection_id: &str, plan_name: String) -> QuotaAccount {
    QuotaAccount {
        id: format!("{connection_id}:volcengine-coding-plan"),
        connection_id: connection_id.to_string(),
        external_id: "volcengine-coding-plan".to_string(),
        display_name: "Coding Plan".to_string(),
        masked_identifier: None,
        plan_name,
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

fn agent_placeholder_account(
    connection_id: &str,
    plan_name: String,
    subscription_until: Option<DateTime<Utc>>,
) -> QuotaAccount {
    QuotaAccount {
        id: format!("{connection_id}:volcengine-afp"),
        connection_id: connection_id.to_string(),
        external_id: "volcengine-afp".to_string(),
        display_name: "Agent Plan AFP".to_string(),
        masked_identifier: None,
        plan_name,
        status: AccountStatus::Unknown,
        windows: Vec::new(),
        critical_window_id: None,
        next_reset_at: None,
        success_count: None,
        failed_count: None,
        recent_requests: Vec::new(),
        subscription_until,
        chatgpt_account_id: None,
        synced_at: Utc::now(),
    }
}

fn critical_window_id(windows: &[QuotaWindow]) -> Option<String> {
    windows
        .iter()
        .filter_map(|window| {
            window
                .remaining_percent
                .map(|percent| (window.id.clone(), percent))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| id)
}

fn canonical_query(params: &[(impl AsRef<str>, String)]) -> String {
    let mut pairs: Vec<(String, String)> = params
        .iter()
        .map(|(key, value)| (key.as_ref().to_string(), value.to_string()))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    pairs
        .into_iter()
        .map(|(key, value)| format!("{}={}", percent_encode(&key), percent_encode(&value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn timestamp_millis(value: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(value).single()
}

fn extract_cookie_value(cookie_header: &str, name: &str) -> Option<String> {
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some((key, value)) = pair.split_once('=') {
            if key.trim() == name {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn build_web_console_action_url(
    base_url: &str,
    service: &str,
    region: &str,
    action: &str,
) -> AppResult<Url> {
    let base_url = if base_url.ends_with('/') {
        base_url.to_string()
    } else {
        format!("{base_url}/")
    };
    let path = format!("{service}/{region}/2024-01-01/{action}");
    Ok(Url::parse(&base_url)?.join(&path)?)
}

fn build_web_coding_usage_url(base_url: &str, service: &str, region: &str) -> AppResult<Url> {
    build_web_console_action_url(base_url, service, region, "GetCodingPlanUsage")
}

fn timestamp_seconds_or_millis(value: i64) -> Option<DateTime<Utc>> {
    if value > 10_000_000_000 {
        Utc.timestamp_millis_opt(value).single()
    } else {
        Utc.timestamp_opt(value, 0).single()
    }
}

fn parse_datetime_value(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(raw) = value.as_str() {
        if let Some(dt) = parse_rfc3339(raw) {
            return Some(dt);
        }
        if let Ok(timestamp) = raw.parse::<i64>() {
            return timestamp_seconds_or_millis(timestamp);
        }
    }
    value
        .as_i64()
        .and_then(timestamp_seconds_or_millis)
        .or_else(|| {
            value
                .as_f64()
                .and_then(|number| timestamp_seconds_or_millis(number as i64))
        })
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn hmac_bytes(key: &[u8], data: &[u8]) -> AppResult<Vec<u8>> {
    let mut mac =
        HmacSha256::new_from_slice(key).map_err(|error| AppError::Message(error.to_string()))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hmac_hex(key: &[u8], data: &[u8]) -> AppResult<String> {
    Ok(hex::encode(hmac_bytes(key, data)?))
}

fn signing_key(secret: &str, date: &str, region: &str, service: &str) -> AppResult<Vec<u8>> {
    let k_date = hmac_bytes(secret.as_bytes(), date.as_bytes())?;
    let k_region = hmac_bytes(&k_date, region.as_bytes())?;
    let k_service = hmac_bytes(&k_region, service.as_bytes())?;
    hmac_bytes(&k_service, b"request")
}

fn mask_ak(value: &str) -> String {
    if value.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****{}", &value[..4], &value[value.len() - 4..])
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn coding_seat_ids(value: Option<&str>) -> Option<Vec<String>> {
    let seat_ids: Vec<String> = value
        .unwrap_or_default()
        .split(|ch| ch == ',' || ch == '，' || ch == '\n')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();
    if seat_ids.is_empty() {
        None
    } else {
        Some(seat_ids)
    }
}

fn default_region() -> String {
    "cn-beijing".to_string()
}

fn default_service() -> String {
    "ark".to_string()
}

fn default_channel() -> String {
    "official".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_matches_volcengine_official_demo() {
        let canonical_request = concat!(
            "GET\n",
            "/\n",
            "Action=ListUsers&Limit=10&Offset=0&Version=2018-01-01\n",
            "host:iam.volcengineapi.com\n",
            "x-date:20240619T071306Z\n",
            "\n",
            "host;x-date\n",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(canonical_request.as_bytes()),
            "5ed5bca3905e1fcbf789abb56a17c2d819674a3bcfa468ae476bd1ea80d135cb"
        );
        let string_to_sign = concat!(
            "HMAC-SHA256\n",
            "20240619T071306Z\n",
            "20240619/cn-beijing/iam/request\n",
            "5ed5bca3905e1fcbf789abb56a17c2d819674a3bcfa468ae476bd1ea80d135cb"
        );
        let key = signing_key(
            "WkRZeE1EQmxPVGhsWWpWak5HVmtNbUUxTXpZeU9UVXlOMlE1TmpZeVlqTQ==",
            "20240619",
            "cn-beijing",
            "iam",
        )
        .unwrap();
        assert_eq!(
            hex::encode(&key),
            "abee62e533a58934c49954459a3c3237d2fccea517c9a7c8a2651d8ea7779826"
        );
        assert_eq!(
            hmac_hex(&key, string_to_sign.as_bytes()).unwrap(),
            "e31c4558bcfe08a286001f59cedbf0791ffd0b2362f10e55ee2627467bcdde93"
        );
    }

    #[test]
    fn parses_get_afp_usage_windows() {
        let raw = r#"
        {
          "ResponseMetadata": { "Action": "GetAFPUsage" },
          "Result": {
            "PlanType": "small",
            "AFPFiveHour": { "Quota": 2000, "Used": 0, "SubscribeTime": -1, "ResetTime": -1 },
            "AFPDaily": { "Quota": 10000, "Used": 0, "SubscribeTime": 1784908800000, "ResetTime": 1784995200000 },
            "AFPWeekly": { "Quota": 7000, "Used": 2628.2977, "SubscribeTime": 1784476800000, "ResetTime": 1785081600000 },
            "AFPMonthly": { "Quota": 20000, "Used": 2628.2977, "SubscribeTime": 1784649599000, "ResetTime": 1787327999000 }
          }
        }
        "#;
        let response: GetAfpUsageResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(response.result.plan_type.as_deref(), Some("small"));
        let windows = build_windows(&response.result);
        assert_eq!(windows.len(), 4);
        assert_eq!(windows[0].name, "5 小时 AFP");
        assert_eq!(windows[0].total, Some(2000.0));
        assert_eq!(windows[0].used, Some(0.0));
        assert_eq!(windows[0].remaining_percent, Some(100.0));
        assert_eq!(windows[2].total, Some(7000.0));
        assert_eq!(windows[2].used, Some(2628.2977));
        assert!(windows[2].remaining_percent.unwrap() > 62.0);
    }

    #[test]
    fn parses_agent_plan_from_get_personal_plan() {
        let raw = r#"
        {
          "ResponseMetadata": { "Action": "GetPersonalPlan" },
          "Result": {
            "PlanType": "small",
            "Status": "Running",
            "StartTime": "2026-05-28T17:38:56Z",
            "EndTime": "2026-08-29T15:59:59Z",
            "AutoRenew": true
          }
        }
        "#;
        let plan: GetPersonalPlanResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(plan.result.plan_type.as_deref(), Some("small"));
        assert_eq!(plan.result.status.as_deref(), Some("Running"));
        assert_eq!(
            plan.result
                .end_time
                .as_deref()
                .and_then(parse_rfc3339)
                .unwrap()
                .to_rfc3339(),
            "2026-08-29T15:59:59+00:00"
        );
    }

    #[test]
    fn parses_list_seat_info_usages_windows() {
        let raw = r#"
        {
          "ResponseMetadata": { "Action": "ListSeatInfoUsages" },
          "Result": {
            "SeatInfoUsages": [
              {
                "SeatID": "S",
                "ProjectName": "demo-project",
                "ShortTermUsage": 567.0622392148214,
                "WeeklyUsage": 158.80747042287504,
                "MonthlyUsage": 460.45462578757065
              }
            ]
          }
        }
        "#;
        let response: ListSeatInfoUsagesResponse = serde_json::from_str(raw).unwrap();
        let usages = extract_seat_info_usages(&response.result);
        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].project_name.as_deref(), Some("demo-project"));
        let windows = build_coding_windows(&usages[0]);
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].name, "短期用量");
        assert_eq!(windows[0].used, Some(567.0622392148214));
        assert_eq!(windows[0].total, Some(100.0));
        assert_eq!(windows[0].unit, QuotaUnit::Percent);
        assert_eq!(windows[0].remaining_percent, Some(0.0));
    }

    #[test]
    fn parses_list_seat_infos_seat_ids() {
        let raw = r#"
        {
          "ResponseMetadata": { "Action": "ListSeatInfos" },
          "Result": {
            "Data": [
              { "SeatID": "seat-abc123", "ProjectName": "default" },
              { "SeatID": "seat-def456", "ProjectName": "default" }
            ],
            "Total": 2
          }
        }
        "#;
        let response: ListSeatInfosResponse = serde_json::from_str(raw).unwrap();
        let seats = extract_seat_infos(&response.result);
        let seat_ids: Vec<_> = seats.into_iter().filter_map(|seat| seat.seat_id).collect();
        assert_eq!(seat_ids, vec!["seat-abc123", "seat-def456"]);
    }

    #[test]
    fn parses_personal_coding_plan_and_usage_details() {
        let plan_raw = r#"
        {
          "ResponseMetadata": { "Action": "GetPersonalPlan" },
          "Result": {
            "PlanType": "Lite",
            "Status": "Running",
            "StartTime": "2026-05-28T17:38:56Z",
            "EndTime": "2026-08-29T15:59:59Z",
            "AutoRenew": true
          }
        }
        "#;
        let plan: GetPersonalPlanResponse = serde_json::from_str(plan_raw).unwrap();
        assert_eq!(plan.result.plan_type.as_deref(), Some("Lite"));
        assert_eq!(plan.result.status.as_deref(), Some("Running"));
        assert!(plan
            .result
            .end_time
            .as_deref()
            .and_then(parse_rfc3339)
            .is_some());

        let usage_raw = r#"
        {
          "ResponseMetadata": { "Action": "GetUsageDetails" },
          "Result": {
            "EndTime": "2026-08-29T15:59:59Z",
            "Details": [
              { "Time": 1783094400000, "ObjectName": "glm-5.2", "Usage": 1000, "Unit": "Tokens", "BillingType": "WithinPlan" },
              { "Time": 1783094400000, "ObjectName": "glm-5.2", "Usage": 200, "Unit": "Tokens", "BillingType": "OutsideOfPlan" }
            ]
          }
        }
        "#;
        let usage: GetUsageDetailsResponse = serde_json::from_str(usage_raw).unwrap();
        let now = Utc.timestamp_millis_opt(1783094400000).single().unwrap();
        let windows =
            build_personal_coding_windows(&usage.result.details, &usage.result.details, now);
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].name, "近 5 小时");
        assert_eq!(windows[0].used, Some(1000.0));
        assert_eq!(windows[0].unit, QuotaUnit::Token);
        assert_eq!(windows[1].name, "近 7 天");
        assert_eq!(windows[1].used, Some(1000.0));
        assert_eq!(windows[2].name, "近 30 天");
        assert_eq!(windows[2].used, Some(1000.0));
        assert_eq!(
            usage_details_subscription_until(&usage.result)
                .unwrap()
                .to_rfc3339(),
            "2026-08-29T15:59:59+00:00"
        );
    }

    #[test]
    fn parses_web_coding_plan_usage_windows() {
        let raw = r#"
        {
          "ResponseMetadata": { "Action": "GetCodingPlanUsage" },
            "Result": {
            "Status": "Running",
            "EndTime": "2026-08-29T15:59:59Z",
            "QuotaUsage": [
              { "Level": "monthly", "Percent": 13.14, "ResetTimestamp": 1787327999 },
              { "Level": "session", "Percent": 0, "ResetTimestamp": -1 },
              { "Level": "weekly", "Percent": 37.55, "ResetTimestamp": 1785081600 }
            ],
            "UpdateTimestamp": 1784995200
          }
        }
        "#;
        let response: WebCodingPlanUsageResponse = serde_json::from_str(raw).unwrap();
        let result = response.result.as_ref().unwrap();
        let windows = build_web_coding_windows(result);
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].name, "当前会话");
        assert_eq!(windows[0].used, Some(0.0));
        assert_eq!(windows[0].remaining_percent, Some(100.0));
        assert_eq!(windows[1].name, "近 1 周");
        assert_eq!(windows[1].unit, QuotaUnit::Percent);
        assert_eq!(windows[1].total, Some(100.0));
        assert!(windows[1].remaining_percent.unwrap() > 62.0);
        assert_eq!(windows[2].name, "近 1 月");
        assert_eq!(
            web_coding_subscription_until(result).unwrap().to_rfc3339(),
            "2026-08-29T15:59:59+00:00"
        );
    }

    #[test]
    fn parses_list_subscribe_trade_subscription_until() {
        let raw = serde_json::json!({
            "ResponseMetadata": { "Action": "ListSubscribeTrade" },
            "Result": {
                "InfoList": [
                    {
                        "ResourceType": "CodingPlan",
                        "ResourceName": "",
                        "BizInfo": "lite",
                        "PayType": "pre",
                        "Status": "Running",
                        "InstanceID": "tsi-20260529013841-8lzd9",
                        "StartTime": "2026-05-28T17:38:56Z",
                        "EndTime": "2026-08-29T15:59:59Z",
                        "EnableAutoRenew": true,
                        "AutoRenewTimes": 1,
                        "RemainAutoRenewNums": -1,
                        "Quantity": 1,
                        "Period": "monthly"
                    }
                ]
            }
        });
        assert_eq!(
            subscribe_trade_subscription_until(raw.pointer("/Result").unwrap())
                .unwrap()
                .to_rfc3339(),
            "2026-08-29T15:59:59+00:00"
        );
    }

    #[test]
    fn canonical_query_encodes_project_values() {
        let query = canonical_query(&[
            ("Version", "2024-01-01".to_string()),
            ("Action", "ListSeatInfoUsages".to_string()),
        ]);
        assert_eq!(query, "Action=ListSeatInfoUsages&Version=2024-01-01");
        assert_eq!(percent_encode("a b/中文"), "a%20b%2F%E4%B8%AD%E6%96%87");
    }

    #[test]
    fn parses_coding_seat_ids() {
        assert_eq!(
            coding_seat_ids(Some("S1,S2，S3\nS4")).unwrap(),
            vec!["S1", "S2", "S3", "S4"]
        );
        assert!(coding_seat_ids(Some("  ")).is_none());
    }

    #[test]
    fn extracts_csrf_token_from_cookie() {
        let cookie = "session=abc; csrfToken=token123; other=xyz";
        assert_eq!(
            extract_cookie_value(cookie, "csrfToken"),
            Some("token123".to_string())
        );
        assert_eq!(extract_cookie_value(cookie, "missing"), None);
    }

    #[test]
    fn builds_web_coding_usage_url_without_dropping_api_top_path() {
        let url = build_web_coding_usage_url(
            "https://console.volcengine.com/api/top",
            "ark",
            "cn-beijing",
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://console.volcengine.com/api/top/ark/cn-beijing/2024-01-01/GetCodingPlanUsage"
        );
    }
}
