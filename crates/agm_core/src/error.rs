use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

/// Stable machine-readable codes for API surfaces (HTTP, Tauri, CLI mapping).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiErrorCode {
    InvalidInput,
    NotFound,
    ConfigInvalid,
    Io,
    Internal,
}

/// Unified API error shape for JSON / `invoke` responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ApiErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: Some(message.into()),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.code)?;
        if let Some(ref m) = self.message {
            write!(f, ": {m}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

/// Errors inside `agm_core` before mapping to [`ApiError`].
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse config TOML: {0}")]
    ConfigDeserialize(#[from] toml::de::Error),

    #[error("failed to serialize config TOML: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    #[error("failed to parse sn_list TOML: {0}")]
    SnListDeserialize(toml::de::Error),

    #[error("invalid locale: {0}")]
    InvalidLocale(String),

    #[error("{0}")]
    Message(String),
}

impl CoreError {
    pub fn api(self) -> ApiError {
        match &self {
            CoreError::Io(_) => ApiError {
                code: ApiErrorCode::Io,
                message: Some(self.to_string()),
                details: None,
            },
            CoreError::ConfigDeserialize(_) | CoreError::ConfigSerialize(_) => ApiError {
                code: ApiErrorCode::ConfigInvalid,
                message: Some(self.to_string()),
                details: None,
            },
            CoreError::SnListDeserialize(_) => ApiError {
                code: ApiErrorCode::ConfigInvalid,
                message: Some(self.to_string()),
                details: None,
            },
            CoreError::InvalidLocale(_) => ApiError {
                code: ApiErrorCode::InvalidInput,
                message: Some(self.to_string()),
                details: None,
            },
            CoreError::Message(_) => ApiError {
                code: ApiErrorCode::InvalidInput,
                message: Some(self.to_string()),
                details: None,
            },
        }
    }
}
