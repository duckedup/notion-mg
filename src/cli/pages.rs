use crate::cli::common::PaginationArgs;
use crate::client::NotionClient;
use crate::client::paginator::{PaginationParams, paginate};
use crate::error::CliError;
use crate::output::{OutputFormat, print_list, print_output};
use clap::Subcommand;
use serde_json::Value;

#[derive(Debug, Subcommand)]
pub enum PagesAction {
    /// Get a page by ID
    Get {
        /// Page ID
        id: String,
    },
    /// Search for pages
    Search {
        /// Search query
        #[arg(long)]
        query: Option<String>,

        #[command(flatten)]
        pagination: PaginationArgs,
    },
    /// Create a new page
    Create {
        /// Parent page or database ID
        #[arg(long)]
        parent_id: String,

        /// Parent type: "page" or "database"
        #[arg(long, default_value = "page")]
        parent_type: String,

        /// Page title
        #[arg(long)]
        title: String,

        /// Page properties as JSON
        #[arg(long)]
        properties: Option<String>,

        /// Page content blocks as JSON
        #[arg(long)]
        children: Option<String>,
    },
    /// Update a page
    Update {
        /// Page ID
        id: String,

        /// Properties as JSON
        #[arg(long)]
        properties: Option<String>,

        /// Archive the page
        #[arg(long)]
        archive: Option<bool>,

        /// Move to trash
        #[arg(long)]
        trash: Option<bool>,
    },
    /// Archive a page
    Archive {
        /// Page ID
        id: String,
    },
}

impl PagesAction {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::Get { id } => {
                let page = client.get_page(id).await?;
                print_output(&page, format)
            }
            Self::Search { query, pagination } => {
                let q = query.clone();
                let pages = paginate(
                    &PaginationParams {
                        page_size: pagination.page_size,
                        start_cursor: pagination.start_cursor.clone(),
                        fetch_all: pagination.all,
                        limit: pagination.limit,
                    },
                    |cursor, size| {
                        let client = client.clone();
                        let q = q.clone();
                        async move { client.search_pages(q.as_deref(), cursor, size).await }
                    },
                )
                .await?;
                print_list(&pages, format)
            }
            Self::Create {
                parent_id,
                parent_type,
                title,
                properties,
                children,
            } => {
                let parent = match parent_type.as_str() {
                    "database" => serde_json::json!({ "database_id": parent_id }),
                    _ => serde_json::json!({ "page_id": parent_id }),
                };

                let mut props = if let Some(p) = properties {
                    serde_json::from_str::<Value>(p).map_err(|e| {
                        CliError::InvalidInput(format!("Invalid properties JSON: {e}"))
                    })?
                } else {
                    serde_json::json!({})
                };

                if parent_type == "database" {
                    props["Name"] = serde_json::json!({
                        "title": crate::api::common::make_rich_text(title)
                    });
                } else {
                    props["title"] = serde_json::json!({
                        "title": crate::api::common::make_rich_text(title)
                    });
                }

                let children_parsed = if let Some(c) = children {
                    Some(serde_json::from_str::<Vec<Value>>(c).map_err(|e| {
                        CliError::InvalidInput(format!("Invalid children JSON: {e}"))
                    })?)
                } else {
                    None
                };

                let page = client
                    .create_page(parent, props, children_parsed, None, None)
                    .await?;
                print_output(&page, format)
            }
            Self::Update {
                id,
                properties,
                archive,
                trash,
            } => {
                let props = if let Some(p) = properties {
                    Some(serde_json::from_str::<Value>(p).map_err(|e| {
                        CliError::InvalidInput(format!("Invalid properties JSON: {e}"))
                    })?)
                } else {
                    None
                };

                let page = client
                    .update_page(id, props, *archive, None, None, *trash)
                    .await?;
                print_output(&page, format)
            }
            Self::Archive { id } => {
                let page = client
                    .update_page(id, None, Some(true), None, None, None)
                    .await?;
                print_output(&page, format)
            }
        }
    }
}
