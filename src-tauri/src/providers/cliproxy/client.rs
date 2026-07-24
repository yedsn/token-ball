use reqwest::{Client, StatusCode};
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

    #[allow(dead_code)]
    pub async fn usage_queue(&self) -> AppResult<Value> {
        self.get_json("v0/management/usage-queue").await
    }

    async fn get_json(&self, path: &str) -> AppResult<Value> {
        let url = self.base_url.join(path)?;
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.management_key))
            .header("x-management-key", &self.management_key)
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::FORBIDDEN
        {
            return Err(AppError::Authentication);
        }
        if !response.status().is_success() {
            return Err(AppError::Message(format!(
                "CLIProxyAPI 返回 HTTP {}",
                response.status()
            )));
        }
        Ok(response.json::<Value>().await?)
    }
}
