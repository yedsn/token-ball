use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::{watch, Mutex, Notify};

use super::{
    config::{self, PhoneBoxSettings},
    payload,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryStatus {
    pub sending: bool,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

pub struct PhoneBoxRuntime {
    config: watch::Sender<PhoneBoxSettings>,
    wake: Notify,
    status: RwLock<DeliveryStatus>,
    pub settings_lock: Mutex<()>,
}

impl Default for PhoneBoxRuntime {
    fn default() -> Self {
        Self {
            config: watch::channel(PhoneBoxSettings::default()).0,
            wake: Notify::new(),
            status: RwLock::new(DeliveryStatus::default()),
            settings_lock: Mutex::new(()),
        }
    }
}

impl PhoneBoxRuntime {
    pub fn configure(&self, settings: PhoneBoxSettings) {
        self.config.send_replace(settings);
    }
    pub fn notify(&self) {
        self.wake.notify_one();
    }
    pub fn status(&self) -> DeliveryStatus {
        self.status.read().unwrap().clone()
    }
    pub fn request_now(&self) -> Result<(), String> {
        if !self.config.borrow().enabled {
            return Err("请先保存并启用 PhoneBox 集成".into());
        }
        self.notify();
        Ok(())
    }
}

pub async fn start(pool: SqlitePool, runtime: Arc<PhoneBoxRuntime>) {
    {
        let _guard = runtime.settings_lock.lock().await;
        match config::load(&pool).await {
            Ok(settings) => runtime.configure(settings),
            Err(_) => {
                runtime.status.write().unwrap().error = Some("PhoneBox 设置无效，请重新保存".into())
            }
        }
    }
    run(
        pool,
        runtime,
        Duration::from_secs(60),
        Duration::from_secs(3),
    )
    .await;
}

async fn run(
    pool: SqlitePool,
    runtime: Arc<PhoneBoxRuntime>,
    heartbeat: Duration,
    timeout: Duration,
) {
    let client = match reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            runtime.status.write().unwrap().error = Some("无法初始化 PhoneBox 客户端".into());
            return;
        }
    };
    let mut config_rx = runtime.config.subscribe();
    let mut immediate = true;
    loop {
        if !immediate {
            tokio::select! {
                _ = config_rx.changed() => {},
                _ = runtime.wake.notified() => {},
                _ = tokio::time::sleep(heartbeat) => {},
            }
        }
        immediate = false;
        let settings = config_rx.borrow_and_update().clone();
        if !settings.enabled {
            runtime.status.write().unwrap().sending = false;
            continue;
        }
        {
            let mut status = runtime.status.write().unwrap();
            status.sending = true;
            status.last_attempt_at = Some(Utc::now());
            status.error = None;
        }
        let attempt = async {
            let bytes = payload::load_payload(&pool)
                .await
                .map_err(|_| "读取额度缓存失败".to_string())?;
            send(&client, &settings, bytes, timeout).await
        };
        let result = tokio::select! {
            biased;
            _ = config_rx.changed() => { immediate = true; None },
            result = attempt => Some(result),
        };
        let mut status = runtime.status.write().unwrap();
        status.sending = false;
        match result {
            Some(Ok(())) => {
                status.last_success_at = Some(Utc::now());
                status.error = None;
            }
            Some(Err(error)) => {
                tracing::warn!(reason = %error, "PhoneBox delivery failed");
                status.error = Some(error);
            }
            None => {
                status.error = None;
            }
        }
    }
}

async fn send(
    client: &reqwest::Client,
    settings: &PhoneBoxSettings,
    bytes: Vec<u8>,
    timeout: Duration,
) -> Result<(), String> {
    let settings = config::normalize(settings).map_err(|_| "PhoneBox 地址无效".to_string())?;
    let request = async {
        let mut response = client
            .post(format!("{}/api/push/tokenball?ttl=180", settings.base_url))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes)
            .send()
            .await
            .map_err(network_error)?;
        if !response.status().is_success() {
            return Err(format!("PhoneBox 返回 HTTP {}", response.status().as_u16()));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(network_error)? {
            if body.len() + chunk.len() > 4096 {
                return Err("PhoneBox 确认响应过大".into());
            }
            body.extend_from_slice(&chunk);
        }
        let value: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| "PhoneBox 确认响应无效".to_string())?;
        if value["ok"] != true || value["key"] != "tokenball" {
            return Err("地址未返回 PhoneBox 推送确认".into());
        }
        Ok(())
    };
    tokio::time::timeout(timeout, request)
        .await
        .map_err(|_| "PhoneBox 请求超时".to_string())?
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "PhoneBox 请求超时"
    } else if error.is_connect() {
        "无法连接 PhoneBox，请确认 Agent 已启动"
    } else {
        "PhoneBox 网络请求失败"
    }
    .into()
}

#[cfg(test)]
mod tests;
