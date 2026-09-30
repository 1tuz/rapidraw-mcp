use rmcp::ErrorData as McpError;
use serde_json::json;

/// RapidRAW API envelope error.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub http_status: Option<u16>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            http_status: None,
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.http_status = Some(status);
        self
    }

    pub fn into_mcp(self) -> McpError {
        McpError::internal_error(
            format!("RapidRAW API error [{}]: {}", self.code, self.message),
            Some(json!({
                "code": self.code,
                "message": self.message,
                "httpStatus": self.http_status,
            })),
        )
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}
