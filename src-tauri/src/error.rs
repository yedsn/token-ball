use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("数据库操作失败: {0}")]
    Database(#[from] sqlx::Error),
    #[error("网络请求失败: {0}")]
    Network(#[from] reqwest::Error),
    #[error("地址格式无效: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("配置目录不可用")]
    DataDirUnavailable,
    #[error("连接不存在")]
    ConnectionNotFound,
    #[error("CLIProxyAPI 认证失败，请检查管理 Key")]
    Authentication,
    #[error("CLIProxyAPI 返回内容无法识别")]
    InvalidResponse,
    #[error("{0}")]
    Message(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl From<AppError> for String {
    fn from(value: AppError) -> Self {
        crate::quota::redact::redact_sensitive(&value.to_string())
    }
}
