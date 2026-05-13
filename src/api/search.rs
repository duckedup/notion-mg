use crate::client::NotionClient;
use crate::error::CliError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<Value>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    #[serde(rename = "type")]
    pub result_type: Option<String>,
}

impl NotionClient {
    pub async fn search(
        &self,
        query: Option<&str>,
        filter: Option<Value>,
        sort: Option<Value>,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<SearchResponse, CliError> {
        let mut body = json!({});

        if let Some(q) = query {
            body["query"] = json!(q);
        }
        if let Some(filter) = filter {
            body["filter"] = filter;
        }
        if let Some(sort) = sort {
            body["sort"] = sort;
        }
        if let Some(cursor) = start_cursor {
            body["start_cursor"] = json!(cursor);
        }
        if let Some(size) = page_size {
            body["page_size"] = json!(size);
        }

        self.post("/search", &body).await
    }
}
