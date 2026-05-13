use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub results: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    #[serde(rename = "type")]
    pub result_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichText {
    #[serde(rename = "type")]
    pub rich_text_type: String,
    pub plain_text: String,
    pub href: Option<String>,
    #[serde(default)]
    pub annotations: Option<Annotations>,
    #[serde(default)]
    pub text: Option<TextContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotations {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub code: bool,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub content: String,
    pub link: Option<Link>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialUser {
    pub object: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parent {
    #[serde(rename = "type")]
    pub parent_type: String,
    #[serde(default)]
    pub database_id: Option<String>,
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub block_id: Option<String>,
    #[serde(default)]
    pub workspace: Option<bool>,
}

pub fn rich_text_to_plain(texts: &[RichText]) -> String {
    texts
        .iter()
        .map(|t| t.plain_text.as_str())
        .collect::<Vec<_>>()
        .join("")
}

pub fn make_rich_text(content: &str) -> Vec<serde_json::Value> {
    vec![serde_json::json!({
        "type": "text",
        "text": { "content": content }
    })]
}
