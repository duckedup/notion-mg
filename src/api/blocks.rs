use crate::api::common::{PaginatedResponse, Parent};
use crate::client::NotionClient;
use crate::error::CliError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub object: String,
    pub id: String,
    #[serde(default)]
    pub parent: Option<Parent>,
    pub created_time: String,
    pub last_edited_time: String,
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(default)]
    pub has_children: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub in_trash: bool,
    #[serde(flatten)]
    pub content: Value,
}

impl Block {
    pub fn plain_text(&self) -> String {
        if let Some(type_data) = self.content.get(&self.block_type)
            && let Some(rich_texts) = type_data.get("rich_text").and_then(|r| r.as_array())
        {
            return rich_texts
                .iter()
                .filter_map(|rt| rt.get("plain_text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("");
        }
        String::new()
    }
}

impl NotionClient {
    pub async fn get_block(&self, block_id: &str) -> Result<Block, CliError> {
        self.get(&format!("/blocks/{block_id}")).await
    }

    pub async fn get_block_children(
        &self,
        block_id: &str,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<PaginatedResponse<Block>, CliError> {
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

        self.get(&format!("/blocks/{block_id}/children{query}"))
            .await
    }

    pub async fn list_all_block_children(&self, block_id: &str) -> Result<Vec<Block>, CliError> {
        crate::client::paginator::paginate(
            &crate::client::paginator::PaginationParams {
                page_size: Some(100),
                start_cursor: None,
                fetch_all: true,
                limit: None,
            },
            |cursor, size| {
                let client = self.clone();
                let block_id = block_id.to_string();
                async move { client.get_block_children(&block_id, cursor, size).await }
            },
        )
        .await
    }

    pub async fn append_block_children(
        &self,
        block_id: &str,
        children: Vec<Value>,
        after: Option<&str>,
    ) -> Result<PaginatedResponse<Block>, CliError> {
        let mut body = json!({ "children": children });

        if let Some(after) = after {
            body["after"] = json!(after);
        }

        self.patch(&format!("/blocks/{block_id}/children"), &body)
            .await
    }

    pub async fn update_block(&self, block_id: &str, content: Value) -> Result<Block, CliError> {
        self.patch(&format!("/blocks/{block_id}"), &content).await
    }

    pub async fn delete_block(&self, block_id: &str) -> Result<Block, CliError> {
        let response: Block = self
            .delete(&format!("/blocks/{block_id}"))
            .await
            .map(|v: Value| serde_json::from_value(v).unwrap())?;
        Ok(response)
    }
}
