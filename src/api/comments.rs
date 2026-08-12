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

/// A comment plus the block it hangs off, so an inline comment can be located in the
/// document it belongs to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchoredComment {
    #[serde(flatten)]
    pub comment: Comment,
    pub anchor_id: String,
    /// Block type the comment is attached to, or `page` for a page-level comment.
    pub anchor_type: String,
    /// Plain text of the anchor block. Empty for page-level comments.
    pub anchor_text: String,
}

/// How deep `fetch_document_comments` descends by default. Notion documents rarely
/// nest further, and each level costs a request per block.
pub const DEFAULT_COMMENT_DEPTH: usize = 10;

struct Anchor {
    id: String,
    kind: String,
    text: String,
    depth: usize,
    has_children: bool,
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
        self.get::<PaginatedResponse<Comment>>(&format!("/comments?{query}"))
            .await
            .map_err(explain_comment_access)
    }

    pub async fn list_all_comments(&self, block_id: &str) -> Result<Vec<Comment>, CliError> {
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
                async move { client.list_comments(&block_id, cursor, size).await }
            },
        )
        .await
    }

    /// Collects every comment on a document: the page-level thread plus the inline
    /// comments anchored to each block, in document order.
    ///
    /// Notion exposes comments only per block, so this walks the block tree and queries
    /// each node. Expect roughly one request per block, against a ~3 req/s rate limit.
    /// Only unresolved comments are returned — the API does not expose resolved ones.
    pub async fn fetch_document_comments(
        &self,
        page_id: &str,
        max_depth: usize,
    ) -> Result<Vec<AnchoredComment>, CliError> {
        let mut found = Vec::new();
        let mut stack = vec![Anchor {
            id: page_id.to_string(),
            kind: "page".to_string(),
            text: String::new(),
            depth: 0,
            has_children: true,
        }];

        while let Some(anchor) = stack.pop() {
            match self.list_all_comments(&anchor.id).await {
                Ok(comments) => found.extend(comments.into_iter().map(|comment| AnchoredComment {
                    comment,
                    anchor_id: anchor.id.clone(),
                    anchor_type: anchor.kind.clone(),
                    anchor_text: anchor.text.clone(),
                })),
                // A block deleted mid-walk should not sink the whole document.
                Err(CliError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }

            if !anchor.has_children || anchor.depth >= max_depth {
                continue;
            }

            let children = match self.list_all_block_children(&anchor.id).await {
                Ok(children) => children,
                Err(CliError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            };

            // Reversed so the stack pops siblings back in document order.
            for block in children.into_iter().rev() {
                stack.push(Anchor {
                    // A child page or database is its own document, not part of this one.
                    has_children: block.has_children
                        && !matches!(block.block_type.as_str(), "child_page" | "child_database"),
                    text: block.plain_text(),
                    kind: block.block_type,
                    id: block.id,
                    depth: anchor.depth + 1,
                });
            }
        }

        Ok(found)
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

        self.post("/comments", &body)
            .await
            .map_err(explain_comment_access)
    }
}

/// Notion returns a bare 403 when the integration lacks comment capabilities, which
/// reads as "this page has no comments" unless the cause is spelled out.
fn explain_comment_access(err: CliError) -> CliError {
    match err {
        CliError::Forbidden(body) => CliError::Forbidden(format!(
            "{body} — the integration needs the \"Read comments\" capability \
             (and \"Insert comments\" to post), enabled at \
             notion.so/profile/integrations, and the page must be shared with it"
        )),
        other => other,
    }
}
