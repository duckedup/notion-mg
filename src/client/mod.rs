pub mod paginator;

use crate::error::CliError;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

const NOTION_API_VERSION: &str = "2022-06-28";
const DEFAULT_BASE_URL: &str = "https://api.notion.com/v1";

#[derive(Debug, Clone)]
pub struct NotionClient {
    client: reqwest::Client,
    base_url: String,
}

impl NotionClient {
    pub fn new(api_key: &str) -> Result<Self, CliError> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    pub fn with_base_url(api_key: &str, base_url: &str) -> Result<Self, CliError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| CliError::Auth(e.to_string()))?,
        );
        headers.insert(
            "Notion-Version",
            HeaderValue::from_static(NOTION_API_VERSION),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, CliError> {
        let url = format!("{}{path}", self.base_url);
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, CliError> {
        let url = format!("{}{path}", self.base_url);
        let response = self.client.post(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    pub async fn patch<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, CliError> {
        let url = format!("{}{path}", self.base_url);
        let response = self.client.patch(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value, CliError> {
        let url = format!("{}{path}", self.base_url);
        let response = self.client.delete(&url).send().await?;
        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, CliError> {
        let status = response.status();
        if status.is_success() {
            let body = response.json::<T>().await?;
            return Ok(body);
        }

        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        match status.as_u16() {
            401 => Err(CliError::Auth(body)),
            404 => Err(CliError::NotFound(body)),
            429 => Err(CliError::RateLimited(body)),
            400 | 422 => Err(CliError::InvalidInput(body)),
            _ => Err(CliError::Api(format!("HTTP {status}: {body}"))),
        }
    }
}
