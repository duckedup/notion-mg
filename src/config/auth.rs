use crate::config::Config;
use crate::error::CliError;

pub fn resolve_api_key(cli_key: Option<&str>, config: &Config) -> Result<String, CliError> {
    if let Some(key) = cli_key {
        return Ok(key.to_string());
    }

    if let Ok(key) = std::env::var("NOTION_API_KEY")
        && !key.is_empty()
    {
        return Ok(key);
    }

    if let Some(key) = &config.api_key
        && !key.is_empty()
    {
        return Ok(key.clone());
    }

    Err(CliError::Auth(
        "No API key found. Set via --api-key, NOTION_API_KEY env var, or run `notion-mg auth init`"
            .to_string(),
    ))
}
