use crate::cli::common::PaginationArgs;
use crate::client::NotionClient;
use crate::client::paginator::{PaginationParams, paginate};
use crate::error::CliError;
use crate::output::{OutputFormat, print_list, print_output};
use clap::Subcommand;
use serde_json::Value;

#[derive(Debug, Subcommand)]
pub enum BlocksAction {
    /// Get a block by ID
    Get {
        /// Block ID
        id: String,
    },
    /// List children of a block (or page)
    Children {
        /// Block or page ID
        id: String,

        #[command(flatten)]
        pagination: PaginationArgs,
    },
    /// Append children blocks to a block or page
    Append {
        /// Block or page ID
        id: String,

        /// Children blocks as JSON array
        #[arg(long)]
        children: String,

        /// Insert after this block ID
        #[arg(long)]
        after: Option<String>,
    },
    /// Update a block
    Update {
        /// Block ID
        id: String,

        /// Block content as JSON
        #[arg(long)]
        content: String,
    },
    /// Delete a block
    Delete {
        /// Block ID
        id: String,
    },
}

impl BlocksAction {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::Get { id } => {
                let block = client.get_block(id).await?;
                print_output(&block, format)
            }
            Self::Children { id, pagination } => {
                let blocks = paginate(
                    &PaginationParams {
                        page_size: pagination.page_size,
                        start_cursor: pagination.start_cursor.clone(),
                        fetch_all: pagination.all,
                        limit: pagination.limit,
                    },
                    |cursor, size| {
                        let client = client.clone();
                        let id = id.clone();
                        async move { client.get_block_children(&id, cursor, size).await }
                    },
                )
                .await?;
                print_list(&blocks, format)
            }
            Self::Append {
                id,
                children,
                after,
            } => {
                let children_val: Vec<Value> = serde_json::from_str(children)
                    .map_err(|e| CliError::InvalidInput(format!("Invalid children JSON: {e}")))?;

                let result = client
                    .append_block_children(id, children_val, after.as_deref())
                    .await?;
                print_list(&result.results, format)
            }
            Self::Update { id, content } => {
                let content_val: Value = serde_json::from_str(content)
                    .map_err(|e| CliError::InvalidInput(format!("Invalid content JSON: {e}")))?;

                let block = client.update_block(id, content_val).await?;
                print_output(&block, format)
            }
            Self::Delete { id } => {
                let block = client.delete_block(id).await?;
                print_output(&block, format)
            }
        }
    }
}
