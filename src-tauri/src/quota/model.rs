use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProviderType {
    CliProxyApi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Unknown,
    Healthy,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AccountStatus {
    Available,
    Warning,
    Cooldown,
    Exhausted,
    Disabled,
    AuthExpired,
    Offline,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuotaUnit {
    Percent,
    Token,
    Credit,
    Afp,
    Request,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PeriodType {
    Rolling,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Custom,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnection {
    pub id: String,
    pub provider_type: ProviderType,
    pub display_name: String,
    pub base_url: String,
    pub enabled: bool,
    pub status: ConnectionStatus,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub masked_management_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub id: Option<String>,
    pub display_name: String,
    pub base_url: String,
    pub management_key: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaAccount {
    pub id: String,
    pub connection_id: String,
    pub external_id: String,
    pub display_name: String,
    pub masked_identifier: Option<String>,
    pub plan_name: String,
    pub status: AccountStatus,
    pub windows: Vec<QuotaWindow>,
    pub critical_window_id: Option<String>,
    pub next_reset_at: Option<DateTime<Utc>>,
    pub success_count: Option<i64>,
    pub failed_count: Option<i64>,
    pub recent_requests: Vec<RequestActivity>,
    pub subscription_until: Option<DateTime<Utc>>,
    pub chatgpt_account_id: Option<String>,
    pub synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestActivity {
    pub time: String,
    pub success: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub id: String,
    pub name: String,
    pub period_type: PeriodType,
    pub period_seconds: Option<i64>,
    pub total: Option<f64>,
    pub used: Option<f64>,
    pub remaining: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub unit: QuotaUnit,
    pub reset_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub is_current_constraint: bool,
    pub data_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSummary {
    pub provider: ProviderType,
    pub total_accounts: usize,
    pub available_accounts: usize,
    pub lowest_remaining_percent: Option<f64>,
    pub nearest_reset_at: Option<DateTime<Utc>>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub stale: bool,
    pub status: ConnectionStatus,
    pub accounts: Vec<QuotaAccount>,
}

impl Default for QuotaSummary {
    fn default() -> Self {
        Self {
            provider: ProviderType::CliProxyApi,
            total_accounts: 0,
            available_accounts: 0,
            lowest_remaining_percent: None,
            nearest_reset_at: None,
            last_synced_at: None,
            stale: false,
            status: ConnectionStatus::Unknown,
            accounts: Vec::new(),
        }
    }
}

impl ToString for ProviderType {
    fn to_string(&self) -> String {
        match self {
            ProviderType::CliProxyApi => "cliproxy_api".to_string(),
        }
    }
}

impl ToString for ConnectionStatus {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl ToString for AccountStatus {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl ToString for QuotaUnit {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl ToString for PeriodType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub fn parse_connection_status(value: &str) -> ConnectionStatus {
    match value {
        "Healthy" => ConnectionStatus::Healthy,
        "Degraded" => ConnectionStatus::Degraded,
        "Failed" => ConnectionStatus::Failed,
        _ => ConnectionStatus::Unknown,
    }
}

pub fn parse_account_status(value: &str) -> AccountStatus {
    match value {
        "Available" => AccountStatus::Available,
        "Warning" => AccountStatus::Warning,
        "Cooldown" => AccountStatus::Cooldown,
        "Exhausted" => AccountStatus::Exhausted,
        "Disabled" => AccountStatus::Disabled,
        "AuthExpired" => AccountStatus::AuthExpired,
        "Offline" => AccountStatus::Offline,
        "Error" => AccountStatus::Error,
        _ => AccountStatus::Unknown,
    }
}

pub fn parse_period_type(value: &str) -> PeriodType {
    match value {
        "Rolling" => PeriodType::Rolling,
        "Hourly" => PeriodType::Hourly,
        "Daily" => PeriodType::Daily,
        "Weekly" => PeriodType::Weekly,
        "Monthly" => PeriodType::Monthly,
        "Custom" => PeriodType::Custom,
        _ => PeriodType::Unknown,
    }
}

pub fn parse_quota_unit(value: &str) -> QuotaUnit {
    match value {
        "Percent" => QuotaUnit::Percent,
        "Token" => QuotaUnit::Token,
        "Credit" => QuotaUnit::Credit,
        "Afp" => QuotaUnit::Afp,
        "Request" => QuotaUnit::Request,
        _ => QuotaUnit::Unknown,
    }
}
