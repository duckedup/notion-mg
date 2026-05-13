use crate::api::common::{PaginatedResponse, Parent, RichText};
use crate::client::NotionClient;
use crate::error::CliError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub object: String,
    pub id: String,
    pub created_time: String,
    pub last_edited_time: String,
    #[serde(default)]
    pub title: Vec<RichText>,
    #[serde(default)]
    pub description: Vec<RichText>,
    pub parent: Parent,
    #[serde(default)]
    pub properties: Value,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub in_trash: bool,
    #[serde(default)]
    pub is_inline: bool,
    #[serde(default)]
    pub icon: Option<Value>,
    #[serde(default)]
    pub cover: Option<Value>,
}

impl Database {
    pub fn title_text(&self) -> String {
        crate::api::common::rich_text_to_plain(&self.title)
    }
}

impl NotionClient {
    pub async fn get_database(&self, database_id: &str) -> Result<Database, CliError> {
        self.get(&format!("/databases/{database_id}")).await
    }

    pub async fn query_database(
        &self,
        database_id: &str,
        filter: Option<Value>,
        sorts: Option<Vec<Value>>,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<PaginatedResponse<crate::api::pages::Page>, CliError> {
        let mut body = json!({});

        if let Some(filter) = filter {
            body["filter"] = filter;
        }
        if let Some(sorts) = sorts {
            body["sorts"] = json!(sorts);
        }
        if let Some(cursor) = start_cursor {
            body["start_cursor"] = json!(cursor);
        }
        if let Some(size) = page_size {
            body["page_size"] = json!(size);
        }

        self.post(&format!("/databases/{database_id}/query"), &body)
            .await
    }

    pub async fn create_database(
        &self,
        parent: Value,
        title: Vec<Value>,
        properties: Value,
    ) -> Result<Database, CliError> {
        let body = json!({
            "parent": parent,
            "title": title,
            "properties": properties,
        });

        self.post("/databases", &body).await
    }

    pub async fn update_database(
        &self,
        database_id: &str,
        title: Option<Vec<Value>>,
        description: Option<Vec<Value>>,
        properties: Option<Value>,
        archived: Option<bool>,
    ) -> Result<Database, CliError> {
        let mut body = json!({});

        if let Some(title) = title {
            body["title"] = json!(title);
        }
        if let Some(description) = description {
            body["description"] = json!(description);
        }
        if let Some(properties) = properties {
            body["properties"] = properties;
        }
        if let Some(archived) = archived {
            body["archived"] = json!(archived);
        }

        self.patch(&format!("/databases/{database_id}"), &body)
            .await
    }

    pub async fn search_databases(
        &self,
        query: Option<&str>,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<PaginatedResponse<Database>, CliError> {
        let mut body = json!({
            "filter": { "value": "database", "property": "object" },
        });

        if let Some(q) = query {
            body["query"] = json!(q);
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
