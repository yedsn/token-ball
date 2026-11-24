use super::*;
use httpmock::{Method::POST, MockServer};

#[tokio::test]
async fn request_contract_and_safe_failures() {
    let server = MockServer::start_async().await;
    let settings = PhoneBoxSettings {
        enabled: true,
        base_url: server.base_url(),
    };
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    for (code, body, success) in [
        (200, r#"{"ok":true,"key":"tokenball"}"#, true),
        (200, "not json PRIVATE_RESPONSE", false),
        (200, r#"{"ok":true,"key":"other"}"#, false),
        (302, "PRIVATE_RESPONSE", false),
        (500, "PRIVATE_RESPONSE", false),
    ] {
        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/api/push/tokenball")
                    .query_param("ttl", "180")
                    .header("content-type", "application/json");
                then.status(code)
                    .header("location", "http://127.0.0.1:1")
                    .body(body);
            })
            .await;
        let result = send(&client, &settings, b"{}".to_vec(), Duration::from_secs(1)).await;
        assert_eq!(result.is_ok(), success);
        assert!(!format!("{result:?}").contains("PRIVATE_RESPONSE"));
        mock.assert_hits_async(1).await;
        mock.delete_async().await;
    }
    let mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200)
                .delay(Duration::from_millis(100))
                .body(r#"{"ok":true,"key":"tokenball"}"#);
        })
        .await;
    assert!(
        send(&client, &settings, b"{}".to_vec(), Duration::from_millis(5))
            .await
            .unwrap_err()
            .contains("超时")
    );
    mock.delete_async().await;
}

async fn database() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    crate::storage::database::run_migrations_for_test(&pool)
        .await
        .unwrap();
    crate::storage::repository::ensure_default_settings(&pool)
        .await
        .unwrap();
    pool
}

async fn settle() {
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
}

async fn wait_until(mut check: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("condition did not become true");
}

#[tokio::test]
async fn burst_is_serial_coalesced_and_disable_cancels_inflight() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200)
                .delay(Duration::from_millis(150))
                .body(r#"{"ok":true,"key":"tokenball"}"#);
        })
        .await;
    let runtime = Arc::new(PhoneBoxRuntime::default());
    runtime.configure(PhoneBoxSettings {
        enabled: true,
        base_url: server.base_url(),
    });
    let task = tokio::spawn(run(
        database().await,
        runtime.clone(),
        Duration::from_secs(60),
        Duration::from_secs(1),
    ));
    wait_until(|| mock.hits() == 1).await;
    for _ in 0..100 {
        runtime.notify();
    }
    tokio::time::sleep(Duration::from_millis(35)).await;
    mock.assert_hits_async(1).await;
    wait_until(|| mock.hits() == 2).await;
    wait_until(|| !runtime.status().sending).await;
    mock.assert_hits_async(2).await;
    assert!(runtime.status().last_success_at.is_some());

    runtime.notify();
    wait_until(|| mock.hits() == 3).await;
    for _ in 0..100 {
        runtime.notify();
    }
    let success = runtime.status().last_success_at;
    runtime.configure(PhoneBoxSettings::default());
    wait_until(|| !runtime.status().sending).await;
    tokio::time::sleep(Duration::from_millis(180)).await;
    mock.assert_hits_async(3).await;
    assert_eq!(runtime.status().last_success_at, success);
    assert!(runtime.status().error.is_none());
    task.abort();
}

#[tokio::test]
async fn new_target_cancels_old_request_without_waiting_for_timeout() {
    let old = MockServer::start_async().await;
    let slow = old
        .mock_async(|when, then| {
            when.method(POST);
            then.status(500).delay(Duration::from_secs(2));
        })
        .await;
    let new = MockServer::start_async().await;
    let fast = new
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200).body(r#"{"ok":true,"key":"tokenball"}"#);
        })
        .await;
    let runtime = Arc::new(PhoneBoxRuntime::default());
    assert!(runtime.status().last_success_at.is_none());
    runtime.configure(PhoneBoxSettings {
        enabled: true,
        base_url: old.base_url(),
    });
    let task = tokio::spawn(run(
        database().await,
        runtime.clone(),
        Duration::from_secs(60),
        Duration::from_secs(3),
    ));
    wait_until(|| slow.hits() == 1).await;
    runtime.configure(PhoneBoxSettings {
        enabled: true,
        base_url: new.base_url(),
    });
    tokio::time::timeout(Duration::from_millis(500), async {
        wait_until(|| runtime.status().last_success_at.is_some()).await;
    })
    .await
    .expect("old request was not cancelled");
    fast.assert_hits_async(1).await;
    assert!(runtime.status().error.is_none());
    task.abort();
}

#[tokio::test]
async fn disabled_sender_and_latest_signal_coalescing() {
    let pool = database().await;
    let runtime = Arc::new(PhoneBoxRuntime::default());
    assert!(runtime.request_now().is_err());
    let task = tokio::spawn(run(
        pool,
        runtime.clone(),
        Duration::from_millis(20),
        Duration::from_millis(10),
    ));
    for _ in 0..100 {
        runtime.notify();
    }
    settle().await;
    assert!(runtime.status().last_attempt_at.is_none());
    runtime.configure(PhoneBoxSettings {
        enabled: true,
        base_url: "http://127.0.0.1:1".into(),
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(runtime.status().last_attempt_at.is_some());
    assert!(runtime.status().error.is_some());
    runtime.configure(PhoneBoxSettings::default());
    settle().await;
    let previous = runtime.status().last_attempt_at;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(runtime.status().last_attempt_at, previous);
    task.abort();
}

#[tokio::test]
async fn startup_retry_recovery_and_target_change() {
    let server = MockServer::start_async().await;
    let mut mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(503);
        })
        .await;
    let pool = database().await;
    let runtime = Arc::new(PhoneBoxRuntime::default());
    runtime.configure(PhoneBoxSettings {
        enabled: true,
        base_url: server.base_url(),
    });
    let task = tokio::spawn(run(
        pool,
        runtime.clone(),
        Duration::from_millis(30),
        Duration::from_millis(100),
    ));
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert!(runtime.status().last_attempt_at.is_some());
    mock.delete_async().await;
    mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200).body(r#"{"ok":true,"key":"tokenball"}"#);
        })
        .await;
    for _ in 0..20 {
        if runtime.status().last_success_at.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(runtime.status().last_success_at.is_some());
    assert!(runtime.status().error.is_none());
    mock.assert_async().await;
    let other = MockServer::start_async().await;
    let received = other
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200).body(r#"{"ok":true,"key":"tokenball"}"#);
        })
        .await;
    runtime.configure(PhoneBoxSettings {
        enabled: true,
        base_url: other.base_url(),
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    received.assert_async().await;
    task.abort();
}
