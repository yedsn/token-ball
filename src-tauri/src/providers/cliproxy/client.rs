use reqwest::{Client, StatusCode};
use serde_json::json;
use serde_json::Value;
use url::Url;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct CliProxyClient {
    client: Client,
    base_url: Url,
    management_key: String,
}

impl CliProxyClient {
    pub fn new(base_url: &str, management_key: &str) -> AppResult<Self> {
        let mut base_url = Url::parse(base_url)?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path().trim_end_matches('/')));
        }
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .build()?,
            base_url,
            management_key: management_key.to_string(),
        })
    }

    pub async fn test_connection(&self) -> AppResult<()> {
        self.get_json("v0/management/usage-statistics-enabled")
            .await
            .map(|_| ())
    }

    pub async fn auth_files(&self) -> AppResult<Value> {
        self.get_json("v0/management/auth-files").await
    }

    pub async fn api_call(
        &self,
        auth_index: &str,
        method: &str,
        url: &str,
        headers: Value,
        data: Option<Value>,
    ) -> AppResult<Value> {
        let mut payload = json!({
            "auth_index": auth_index,
            "method": method,
            "url": url,
            "header": headers,
        });
        if let Some(data) = data {
            payload["data"] = match data {
                Value::String(raw) => Value::String(raw),
                other => Value::String(other.to_string()),
            };
        }
        self.post_json("v0/management/api-call", payload).await
    }

    #[allow(dead_code)]
    pub async fn usage_queue(&self) -> AppResult<Value> {
        self.get_json("v0/management/usage-queue").await
    }

    async fn get_json(&self, path: &str) -> AppResult<Value> {
        let url = self.base_url.join(path)?;
        let response = self
            .client
            .get(url)
            .header("X-Management-Key", &self.management_key)
            .header("Authorization", format!("Bearer {}", self.management_key))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if body.to_ascii_lowercase().contains("ip banned") {
                return Err(AppError::Message(format!(
                    "CLIProxyAPI 管理接口临时封禁当前 IP：{}",
                    body
                )));
            }
            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                if body.trim().is_empty() {
                    return Err(AppError::Authentication);
                }
                return Err(AppError::Message(format!(
                    "CLIProxyAPI 认证失败，接口返回 HTTP {}：{}",
                    status, body
                )));
            }
            return Err(AppError::Message(format!(
                "CLIProxyAPI 返回 HTTP {}：{}",
                status, body
            )));
        }
        Ok(response.json::<Value>().await?)
    }

    async fn post_json(&self, path: &str, payload: Value) -> AppResult<Value> {
        let url = self.base_url.join(path)?;
        let response = self
            .client
            .post(url)
            .header("X-Management-Key", &self.management_key)
            .header("Authorization", format!("Bearer {}", self.management_key))
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Message(format!(
                "CLIProxyAPI 返回 HTTP {}：{}",
                status, body
            )));
        }
        Ok(response.json::<Value>().await?)
    }
}
