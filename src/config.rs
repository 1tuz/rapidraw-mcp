use std::env;
use std::time::Duration;

const DEFAULT_API_URL: &str = "http://127.0.0.1:17355";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub api_token: Option<String>,
    pub timeout: Duration,
}

impl Config {
    pub fn from_env() -> Self {
        let api_url = env::var("RAPIDRAW_API_URL")
            .unwrap_or_else(|_| DEFAULT_API_URL.to_string())
            .trim_end_matches('/')
            .to_string();

        let api_token = env::var("RAPIDRAW_API_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let timeout_secs = env::var("RAPIDRAW_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        Self {
            api_url,
            api_token,
            timeout: Duration::from_secs(timeout_secs.max(1)),
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.api_url = url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.api_token = token.filter(|s| !s.is_empty());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            api_token: None,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let c = Config::default();
        assert_eq!(c.api_url, "http://127.0.0.1:17355");
        assert!(c.api_token.is_none());
        assert_eq!(c.timeout, Duration::from_secs(30));
    }

    #[test]
    fn with_url_strips_trailing_slash() {
        let c = Config::default().with_url("http://127.0.0.1:9999/");
        assert_eq!(c.api_url, "http://127.0.0.1:9999");
    }
}
