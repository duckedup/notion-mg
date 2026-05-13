use crate::api::common::{PaginatedResponse, Parent, PartialUser, RichText};
use crate::client::NotionClient;
use crate::error::CliError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub object: String,
    pub id: String,
    pub parent: Parent,
    #[serde(default)]
    pub discussion_id: Option<String>,
    pub created_time: String,
    #[serde(default)]
    pub last_edited_time: Option<String>,
    #[serde(default)]
    pub created_by: Option<PartialUser>,
    #[serde(default)]
    pub rich_text: Vec<RichText>,
}

impl Comment {
    pub fn plain_text(&self) -> String {
        crate::api::common::rich_text_to_plain(&self.rich_text)
    }
}

impl NotionClient {
    pub async fn list_comments(
        &self,
        block_id: &str,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<PaginatedResponse<Comment>, CliError> {
        let mut params = vec![format!("block_id={block_id}")];
        if let Some(cursor) = start_cursor {
            params.push(format!("start_cursor={cursor}"));
        }
        if let Some(size) = page_size {
            params.push(format!("page_size={size}"));
        }

        let query = params.join("&");
        self.get(&format!("/comments?{query}")).await
    }

    pub async fn create_comment(
        &self,
        parent: Value,
        rich_text: Vec<Value>,
        discussion_id: Option<&str>,
    ) -> Result<Comment, CliError> {
        let mut body = json!({
            "parent": parent,
            "rich_text": rich_text,
        });

        if let Some(discussion_id) = discussion_id {
            body["discussion_id"] = json!(discussion_id);
        }

        self.post("/comments", &body).await
    }
}
