use crate::api::common::PaginatedResponse;
use crate::client::NotionClient;
use crate::error::CliError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub object: String,
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub user_type: Option<String>,
    #[serde(default)]
    pub person: Option<serde_json::Value>,
    #[serde(default)]
    pub bot: Option<serde_json::Value>,
}

impl NotionClient {
    pub async fn list_users(
        &self,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<PaginatedResponse<User>, CliError> {
        let mut params = Vec::new();
        if let Some(cursor) = start_cursor {
            params.push(format!("start_cursor={cursor}"));
        }
        if let Some(size) = page_size {
            params.push(format!("page_size={size}"));
        }

        let query = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        self.get(&format!("/users{query}")).await
    }

    pub async fn get_user(&self, user_id: &str) -> Result<User, CliError> {
        self.get(&format!("/users/{user_id}")).await
    }

    pub async fn get_me(&self) -> Result<User, CliError> {
        self.get("/users/me").await
    }
}
