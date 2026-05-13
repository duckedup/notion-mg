use crate::api::common::{PaginatedResponse, Parent, PartialUser, RichText};
use crate::client::NotionClient;
use crate::error::CliError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub object: String,
    pub id: String,
    pub created_time: String,
    pub last_edited_time: String,
    #[serde(default)]
    pub created_by: Option<PartialUser>,
    #[serde(default)]
    pub last_edited_by: Option<PartialUser>,
    pub parent: Parent,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub in_trash: bool,
    #[serde(default)]
    pub properties: Value,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub public_url: Option<String>,
    #[serde(default)]
    pub icon: Option<Value>,
    #[serde(default)]
    pub cover: Option<Value>,
}

impl Page {
    pub fn title(&self) -> String {
        if let Value::Object(props) = &self.properties {
            for (_key, val) in props {
                if val.get("type").and_then(|t| t.as_str()) == Some("title")
                    && let Some(title_arr) = val.get("title").and_then(|t| t.as_array())
                {
                    let texts: Vec<RichText> = title_arr
                        .iter()
                        .filter_map(|t| serde_json::from_value(t.clone()).ok())
                        .collect();
                    return crate::api::common::rich_text_to_plain(&texts);
                }
            }
        }
        String::new()
    }
}

impl NotionClient {
    pub async fn get_page(&self, page_id: &str) -> Result<Page, CliError> {
        self.get(&format!("/pages/{page_id}")).await
    }

    pub async fn create_page(
        &self,
        parent: Value,
        properties: Value,
        children: Option<Vec<Value>>,
        icon: Option<Value>,
        cover: Option<Value>,
    ) -> Result<Page, CliError> {
        let mut body = json!({
            "parent": parent,
            "properties": properties,
        });

        if let Some(children) = children {
            body["children"] = json!(children);
        }
        if let Some(icon) = icon {
            body["icon"] = icon;
        }
        if let Some(cover) = cover {
            body["cover"] = cover;
        }

        self.post("/pages", &body).await
    }

    pub async fn update_page(
        &self,
        page_id: &str,
        properties: Option<Value>,
        archived: Option<bool>,
        icon: Option<Value>,
        cover: Option<Value>,
        in_trash: Option<bool>,
    ) -> Result<Page, CliError> {
        let mut body = json!({});

        if let Some(properties) = properties {
            body["properties"] = properties;
        }
        if let Some(archived) = archived {
            body["archived"] = json!(archived);
        }
        if let Some(icon) = icon {
            body["icon"] = icon;
        }
        if let Some(cover) = cover {
            body["cover"] = cover;
        }
        if let Some(in_trash) = in_trash {
            body["in_trash"] = json!(in_trash);
        }

        self.patch(&format!("/pages/{page_id}"), &body).await
    }

    pub async fn search_pages(
        &self,
        query: Option<&str>,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<PaginatedResponse<Page>, CliError> {
        let mut body = json!({
            "filter": { "value": "page", "property": "object" },
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
