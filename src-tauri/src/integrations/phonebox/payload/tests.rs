use super::*;
use crate::quota::{ConnectionInput, ProviderType};

async fn fixture() -> (
    SqlitePool,
    Vec<ProviderConnection>,
    Vec<QuotaAccount>,
    HashMap<String, Option<DateTime<Utc>>>,
) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    crate::storage::database::run_migrations_for_test(&pool)
        .await
        .unwrap();
    repository::ensure_default_settings(&pool).await.unwrap();
    let mut connections = Vec::new();
    let mut accounts = Vec::new();
    let mut collected = HashMap::new();
    for (i, provider) in [
        ProviderType::CliProxyApi,
        ProviderType::Volcengine,
        ProviderType::Qianwen,
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("connection-{i}");
        repository::save_connection(
            &pool,
            ConnectionInput {
                id: Some(id.clone()),
                provider_type: provider,
                display_name: "PRIVATE_INSTANCE".into(),
                base_url: "http://PRIVATE_HOST".into(),
                management_key: if i == 0 {
                    "PRIVATE_KEY".into()
                } else {
                    "{\"qianwenCookie\":\"PRIVATE_COOKIE\",\"secretAccessKey\":\"PRIVATE_SK\"}"
                        .into()
                },
                enabled: Some(true),
            },
        )
        .await
        .unwrap();
        repository::update_connection_status(
            &pool,
            &id,
            ConnectionStatus::Healthy,
            Some(Utc::now()),
        )
        .await
        .unwrap();
        let mut account = crate::providers::cliproxy::mapper::map_auth_files(&id, &json!([{"id":format!("PRIVATE_ACCOUNT_{i}"), "email":"PRIVATE_EMAIL", "plan":"Codex Plus"}])).remove(0);
        account.status = AccountStatus::Available;
        let mut window: QuotaWindow = serde_json::from_value(json!({
            "id":"afp-five-hour", "name":"5 小时 AFP", "periodType":"rolling", "periodSeconds":null,
            "total":100, "used":20, "remaining":80, "remainingPercent":80, "unit":"percent",
            "resetAt":null, "isActive":true, "isCurrentConstraint":true, "dataSource":"PRIVATE_SOURCE" })).unwrap();
        window.id = format!("afp-five-hour-{i}");
        window.remaining_percent = [Some(80.0), Some(20.0), None][i];
        account.critical_window_id = Some(window.id.clone());
        account.windows = vec![window];
        account.chatgpt_account_id = Some("PRIVATE_CHATGPT".into());
        repository::replace_connection_accounts(&pool, &id, &[account.clone()])
            .await
            .unwrap();
        collected.insert(account.id.clone(), Some(account.synced_at));
        accounts.push(account);
    }
    connections.extend(repository::list_connections(&pool).await.unwrap());
    (pool, connections, accounts, collected)
}

#[tokio::test]
async fn mixed_scope_arithmetic_and_privacy() {
    let (_, mut connections, accounts, collected) = fixture().await;
    let payload = build_payload(
        &connections,
        &accounts,
        &collected,
        &DisplaySettings::default(),
        Utc::now(),
    );
    assert_eq!(payload["summary"]["averageRemainingPercent"], 50.0);
    assert_eq!(payload["summary"]["equivalentAccounts"], 1.0);
    assert_eq!(payload["summary"]["knownQuotaAccountCount"], 2);
    // Display names are intentionally shared; no other source identifiers are.
    let mut private_check = payload.clone();
    for row in private_check["accounts"].as_array_mut().unwrap() {
        row.as_object_mut().unwrap().remove("displayName");
    }
    let text = String::from_utf8(encode(private_check).unwrap()).unwrap();
    assert!(!text.contains("PRIVATE_"), "private source fields leaked");
    assert!(payload["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .all(|a| a["displayName"].is_string()));
    let providers: Vec<_> = payload["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["provider"].as_str().unwrap())
        .collect();
    for expected in ["cliProxyApi", "volcengine", "qianwen"] {
        assert!(providers.contains(&expected));
    }
    for connection in &mut connections {
        connection.enabled = false;
    }
    let empty = build_payload(
        &connections,
        &accounts,
        &collected,
        &DisplaySettings::default(),
        Utc::now(),
    );
    assert_eq!(empty["summary"]["accountCount"], 0);
    assert!(empty["summary"]["averageRemainingPercent"].is_null());
    assert!(empty["summary"]["equivalentAccounts"].is_null());
    for connection in &mut connections {
        connection.enabled = true;
    }
    let mut unknown = accounts.clone();
    for account in &mut unknown {
        account.windows[0].remaining_percent = None;
    }
    let unknown = build_payload(
        &connections,
        &unknown,
        &collected,
        &DisplaySettings::default(),
        Utc::now(),
    );
    assert_eq!(unknown["summary"]["accountCount"], 3);
    assert!(unknown["summary"]["averageRemainingPercent"].is_null());
    assert!(unknown["summary"]["equivalentAccounts"].is_null());
}

#[tokio::test]
async fn tray_priority_and_display_name_are_explicit_and_bounded() {
    let (_, connections, mut accounts, collected) = fixture().await;
    let now = Utc::now();
    let account = &mut accounts[0];
    account.display_name = "正式账号 <团队>\n".repeat(30);
    account.next_reset_at = Some(now + Duration::hours(1));
    assert_eq!(priority_reset(account), account.next_reset_at);
    account.windows[0].reset_at = Some(now + Duration::hours(2));
    let mut weekly = account.windows[0].clone();
    weekly.period_type = PeriodType::Weekly;
    weekly.reset_at = Some(now + Duration::days(3));
    let mut monthly = weekly.clone();
    monthly.period_type = PeriodType::Monthly;
    monthly.reset_at = Some(now + Duration::days(20));
    account.windows.extend([weekly, monthly.clone()]);
    assert_eq!(priority_reset(account), monthly.reset_at);
    let mut custom = monthly.clone();
    custom.period_type = PeriodType::Custom;
    custom.id = "custom".into();
    custom.name = "custom".into();
    custom.period_seconds = Some(9999999);
    custom.reset_at = Some(now + Duration::days(40));
    account.windows.push(custom.clone());
    assert_eq!(
        priority_reset(account),
        monthly.reset_at,
        "monthly wins over larger custom window"
    );
    account
        .windows
        .retain(|w| w.period_type != PeriodType::Monthly);
    assert_eq!(priority_reset(account), custom.reset_at);
    let payload = build_payload(
        &connections,
        &accounts,
        &collected,
        &DisplaySettings::default(),
        now,
    );
    let row = payload["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == opaque(&accounts[0].id))
        .unwrap();
    assert_eq!(row["displayName"].as_str().unwrap().chars().count(), 160);
    assert!(!row["displayName"].as_str().unwrap().contains('\n'));
    assert_eq!(row["priorityResetAt"], json!(custom.reset_at));
    assert_eq!(
        row["periods"]["fiveHour"]["resetAt"],
        json!(accounts[0].windows[0].reset_at)
    );
}

#[tokio::test]
async fn storage_scope_failure_and_old_cache_are_independent() {
    let (pool, _, accounts, _) = fixture().await;
    repository::update_connection_status(&pool, "connection-0", ConnectionStatus::Degraded, None)
        .await
        .unwrap();
    let first: Value = serde_json::from_slice(&load_payload(&pool).await.unwrap()).unwrap();
    let row = first["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == opaque(&accounts[0].id))
        .unwrap();
    assert_eq!(row["connectionStatus"], "degraded");
    assert_eq!(row["stale"], true);
    assert_eq!(row["periods"]["fiveHour"]["remainingPercent"], 80.0);
    let time = row["collectedAt"].clone();
    repository::set_connection_enabled(&pool, "connection-1", false)
        .await
        .unwrap();
    let next: Value = serde_json::from_slice(&load_payload(&pool).await.unwrap()).unwrap();
    assert_eq!(next["summary"]["accountCount"], 2);
    let row = next["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == opaque(&accounts[0].id))
        .unwrap();
    assert_eq!(row["collectedAt"], time);
    assert_eq!(row["stale"], true);
    assert_eq!(repository::load_accounts(&pool).await.unwrap().len(), 3);
}

#[tokio::test]
async fn periods_freshness_and_missing_storage_timestamp() {
    let (pool, connections, mut accounts, mut collected) = fixture().await;
    let now = Utc::now();
    let account = &mut accounts[0];
    let time = now + Duration::hours(1);
    account.windows[0].reset_at = Some(time);
    let mut weekly = account.windows[0].clone();
    weekly.id = "weekly".into();
    weekly.period_type = PeriodType::Weekly;
    weekly.reset_at = None;
    account.windows.push(weekly);
    account.subscription_until = Some(now + Duration::days(30));
    collected.insert(account.id.clone(), Some(now - Duration::seconds(4000)));
    let account_id = account.id.clone();
    let mut settings = DisplaySettings::default();
    settings.random_sync_delay_enabled = true;
    settings.random_sync_delay_max_seconds = 600;
    let fresh = build_payload(&connections, &accounts, &collected, &settings, now);
    let row = fresh["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == opaque(&account_id))
        .unwrap();
    assert_eq!(row["periods"]["fiveHour"]["resetAt"], json!(time));
    assert!(row["periods"]["weekly"]["resetAt"].is_null());
    assert!(row["periods"]["monthly"].is_null());
    assert_eq!(row["stale"], false);
    settings.random_sync_delay_enabled = false;
    let stale = build_payload(&connections, &accounts, &collected, &settings, now);
    assert_eq!(stale["summary"]["stale"], true);
    sqlx::query("DELETE FROM quota_snapshots WHERE account_id = ?1")
        .bind(&accounts[0].id)
        .execute(&pool)
        .await
        .unwrap();
    let stored: Value = serde_json::from_slice(&load_payload(&pool).await.unwrap()).unwrap();
    let row = stored["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == opaque(&accounts[0].id))
        .unwrap();
    assert!(row["collectedAt"].is_null());
    assert_eq!(row["stale"], true);
}

#[tokio::test]
async fn bounded_detail_keeps_full_summary() {
    let (_, connections, mut accounts, collected) = fixture().await;
    let original = accounts[0].clone();
    for i in 0..300 {
        let mut account = original.clone();
        account.id = format!("{i}");
        accounts.push(account);
    }
    let bytes = encode(build_payload(
        &connections,
        &accounts,
        &collected,
        &DisplaySettings::default(),
        Utc::now(),
    ))
    .unwrap();
    assert!(bytes.len() <= MAX_BYTES);
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["summary"]["accountCount"], 303);
    assert!(value["omittedAccountCount"].as_u64().unwrap() > 0);
    assert_eq!(
        value["includedAccountCount"].as_u64().unwrap()
            + value["omittedAccountCount"].as_u64().unwrap(),
        303
    );
}

#[tokio::test]
async fn period_aliases_follow_desktop_priority() {
    let (_, _, accounts, _) = fixture().await;
    for (name, expected) in [
        ("5h", Some("fiveHour")),
        ("5 h", Some("fiveHour")),
        ("5 小时 AFP", Some("fiveHour")),
        ("每周", Some("weekly")),
        ("近 1 周", Some("weekly")),
        ("7d", Some("weekly")),
        ("每月", Some("monthly")),
        ("近 1 月", Some("monthly")),
        ("30d", Some("monthly")),
        ("15h", None),
        ("15 小时", None),
        ("17d", None),
        ("130d", None),
        ("当前会话", None),
    ] {
        let mut window = accounts[0].windows[0].clone();
        window.id = "unknown".into();
        window.name = name.into();
        assert_eq!(period(&window), expected, "{name}");
        window.period_type = PeriodType::Daily;
        assert_eq!(period(&window), None);
        window.period_type = PeriodType::Weekly;
        assert_eq!(period(&window), Some("weekly"));
    }
    assert_eq!(percent(Some(f64::NAN)), None);
    assert_eq!(percent(Some(101.0)), None);
    assert_eq!(parse_time("not-a-date"), None);
}
