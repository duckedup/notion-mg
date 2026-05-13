use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("{0}")]
    General(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Auth(_) => 2,
            Self::RateLimited(_) => 3,
            Self::NotFound(_) => 4,
            Self::InvalidInput(_) => 5,
            Self::Api(_) => 6,
            Self::General(_) => 1,
        }
    }

    pub fn error_type(&self) -> &str {
        match self {
            Self::Auth(_) => "auth_error",
            Self::RateLimited(_) => "rate_limited",
            Self::NotFound(_) => "not_found",
            Self::InvalidInput(_) => "invalid_input",
            Self::Api(_) => "api_error",
            Self::General(_) => "error",
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "error": {
                "type": self.error_type(),
                "message": self.to_string(),
                "exit_code": self.exit_code(),
            }
        })
    }
}

impl From<reqwest::Error> for CliError {
    fn from(err: reqwest::Error) -> Self {
        Self::Api(err.to_string())
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::General(err.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        Self::Api(format!("JSON error: {err}"))
    }
}

impl From<toml::de::Error> for CliError {
    fn from(err: toml::de::Error) -> Self {
        Self::General(format!("Config parse error: {err}"))
    }
}

impl From<toml::ser::Error> for CliError {
    fn from(err: toml::ser::Error) -> Self {
        Self::General(format!("Config write error: {err}"))
    }
}
