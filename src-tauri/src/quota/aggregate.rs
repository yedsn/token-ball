use super::{AccountStatus, ConnectionStatus, ProviderType, QuotaAccount, QuotaSummary};

pub fn build_summary(
    accounts: Vec<QuotaAccount>,
    status: ConnectionStatus,
    stale: bool,
) -> QuotaSummary {
    let total_accounts = accounts.len();
    let available_accounts = accounts
        .iter()
        .filter(|account| {
            matches!(
                account.status,
                AccountStatus::Available | AccountStatus::Warning
            )
        })
        .count();

    let lowest_remaining_percent = accounts
        .iter()
        .filter_map(|account| {
            account
                .critical_window_id
                .as_ref()
                .and_then(|id| account.windows.iter().find(|window| &window.id == id))
                .and_then(|window| window.remaining_percent)
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let nearest_reset_at = accounts
        .iter()
        .flat_map(|account| account.windows.iter())
        .filter_map(|window| window.reset_at)
        .min();

    let last_synced_at = accounts.iter().map(|account| account.synced_at).max();

    QuotaSummary {
        provider: ProviderType::CliProxyApi,
        total_accounts,
        available_accounts,
        lowest_remaining_percent,
        nearest_reset_at,
        last_synced_at,
        stale,
        status,
        accounts,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::quota::{
        build_summary, AccountStatus, ConnectionStatus, PeriodType, QuotaAccount, QuotaUnit,
        QuotaWindow,
    };

    #[test]
    fn unknown_quota_is_not_zero() {
        let summary = build_summary(
            vec![QuotaAccount {
                id: "a".to_string(),
                connection_id: "c".to_string(),
                external_id: "e".to_string(),
                display_name: "Account".to_string(),
                masked_identifier: None,
                plan_name: "Codex Plus".to_string(),
                status: AccountStatus::Unknown,
                windows: vec![QuotaWindow {
                    id: "w".to_string(),
                    name: "Unknown".to_string(),
                    period_type: PeriodType::Unknown,
                    period_seconds: None,
                    total: None,
                    used: None,
                    remaining: None,
                    remaining_percent: None,
                    unit: QuotaUnit::Unknown,
                    reset_at: None,
                    is_active: true,
                    is_current_constraint: false,
                    data_source: "test".to_string(),
                }],
                critical_window_id: None,
                next_reset_at: None,
                success_count: None,
                failed_count: None,
                recent_requests: Vec::new(),
                subscription_until: None,
                chatgpt_account_id: None,
                synced_at: Utc::now(),
            }],
            ConnectionStatus::Healthy,
            false,
        );

        assert_eq!(summary.lowest_remaining_percent, None);
    }
}
