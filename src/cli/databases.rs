use crate::cli::common::PaginationArgs;
use crate::client::NotionClient;
use crate::client::paginator::{PaginationParams, paginate};
use crate::error::CliError;
use crate::output::{OutputFormat, print_list, print_output};
use clap::Subcommand;
use serde_json::Value;

#[derive(Debug, Subcommand)]
pub enum DatabasesAction {
    /// Get a database by ID
    Get {
        /// Database ID
        id: String,
    },
    /// Query a database
    Query {
        /// Database ID
        id: String,

        /// Filter as JSON
        #[arg(long)]
        filter: Option<String>,

        /// Sorts as JSON array
        #[arg(long)]
        sorts: Option<String>,

        #[command(flatten)]
        pagination: PaginationArgs,
    },
    /// Search for databases
    Search {
        /// Search query
        #[arg(long)]
        query: Option<String>,

        #[command(flatten)]
        pagination: PaginationArgs,
    },
    /// Create a database
    Create {
        /// Parent page ID
        #[arg(long)]
        parent_id: String,

        /// Database title
        #[arg(long)]
        title: String,

        /// Properties schema as JSON
        #[arg(long)]
        properties: String,
    },
    /// Update a database
    Update {
        /// Database ID
        id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// Properties schema as JSON
        #[arg(long)]
        properties: Option<String>,

        /// Archive the database
        #[arg(long)]
        archive: Option<bool>,
    },
}

impl DatabasesAction {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::Get { id } => {
                let db = client.get_database(id).await?;
                print_output(&db, format)
            }
            Self::Query {
                id,
                filter,
                sorts,
                pagination,
            } => {
                let filter_val =
                    if let Some(f) = filter {
                        Some(serde_json::from_str::<Value>(f).map_err(|e| {
                            CliError::InvalidInput(format!("Invalid filter JSON: {e}"))
                        })?)
                    } else {
                        None
                    };

                let sorts_val =
                    if let Some(s) = sorts {
                        Some(serde_json::from_str::<Vec<Value>>(s).map_err(|e| {
                            CliError::InvalidInput(format!("Invalid sorts JSON: {e}"))
                        })?)
                    } else {
                        None
                    };

                let pages = paginate(
                    &PaginationParams {
                        page_size: pagination.page_size,
                        start_cursor: pagination.start_cursor.clone(),
                        fetch_all: pagination.all,
                        limit: pagination.limit,
                    },
                    |cursor, size| {
                        let client = client.clone();
                        let id = id.clone();
                        let filter_val = filter_val.clone();
                        let sorts_val = sorts_val.clone();
                        async move {
                            client
                                .query_database(&id, filter_val, sorts_val, cursor, size)
                                .await
                        }
                    },
                )
                .await?;
                print_list(&pages, format)
            }
            Self::Search { query, pagination } => {
                let q = query.clone();
                let dbs = paginate(
                    &PaginationParams {
                        page_size: pagination.page_size,
                        start_cursor: pagination.start_cursor.clone(),
                        fetch_all: pagination.all,
                        limit: pagination.limit,
                    },
                    |cursor, size| {
                        let client = client.clone();
                        let q = q.clone();
                        async move { client.search_databases(q.as_deref(), cursor, size).await }
                    },
                )
                .await?;
                print_list(&dbs, format)
            }
            Self::Create {
                parent_id,
                title,
                properties,
            } => {
                let parent = serde_json::json!({ "page_id": parent_id });
                let title_val = crate::api::common::make_rich_text(title);
                let props = serde_json::from_str::<Value>(properties)
                    .map_err(|e| CliError::InvalidInput(format!("Invalid properties JSON: {e}")))?;

                let db = client.create_database(parent, title_val, props).await?;
                print_output(&db, format)
            }
            Self::Update {
                id,
                title,
                description,
                properties,
                archive,
            } => {
                let title_val = title
                    .as_ref()
                    .map(|t| crate::api::common::make_rich_text(t));
                let desc_val = description
                    .as_ref()
                    .map(|d| crate::api::common::make_rich_text(d));
                let props = if let Some(p) = properties {
                    Some(serde_json::from_str::<Value>(p).map_err(|e| {
                        CliError::InvalidInput(format!("Invalid properties JSON: {e}"))
                    })?)
                } else {
                    None
                };

                let db = client
                    .update_database(id, title_val, desc_val, props, *archive)
                    .await?;
                print_output(&db, format)
            }
        }
    }
}
