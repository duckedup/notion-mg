use crate::api::comments::DEFAULT_COMMENT_DEPTH;
use crate::cli::common::PaginationArgs;
use crate::client::NotionClient;
use crate::client::paginator::{PaginationParams, paginate};
use crate::error::CliError;
use crate::output::{OutputFormat, print_list, print_output};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum CommentsAction {
    /// List comments on a page or block
    List {
        /// Page or block ID
        #[arg(long)]
        block_id: String,

        #[command(flatten)]
        pagination: PaginationArgs,
    },
    /// List every comment on a document: page-level plus inline comments on each block
    Document {
        /// Page ID
        #[arg(long)]
        page_id: String,

        /// How far to descend into nested blocks (0 = page-level comments only)
        #[arg(long, default_value_t = DEFAULT_COMMENT_DEPTH)]
        max_depth: usize,
    },
    /// Create a comment on a page
    Create {
        /// Page ID
        #[arg(long)]
        page_id: String,

        /// Comment text
        #[arg(long)]
        text: String,

        /// Discussion ID (to reply to an existing thread)
        #[arg(long)]
        discussion_id: Option<String>,
    },
}

impl CommentsAction {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::List {
                block_id,
                pagination,
            } => {
                let comments = paginate(
                    &PaginationParams {
                        page_size: pagination.page_size,
                        start_cursor: pagination.start_cursor.clone(),
                        fetch_all: pagination.all,
                        limit: pagination.limit,
                    },
                    |cursor, size| {
                        let client = client.clone();
                        let block_id = block_id.clone();
                        async move { client.list_comments(&block_id, cursor, size).await }
                    },
                )
                .await?;
                print_list(&comments, format)
            }
            Self::Document { page_id, max_depth } => {
                let comments = client.fetch_document_comments(page_id, *max_depth).await?;
                print_list(&comments, format)
            }
            Self::Create {
                page_id,
                text,
                discussion_id,
            } => {
                let parent = serde_json::json!({ "page_id": page_id });
                let rich_text = crate::api::common::make_rich_text(text);

                let comment = client
                    .create_comment(parent, rich_text, discussion_id.as_deref())
                    .await?;
                print_output(&comment, format)
            }
        }
    }
}
