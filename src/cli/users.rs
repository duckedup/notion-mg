use crate::cli::common::PaginationArgs;
use crate::client::NotionClient;
use crate::client::paginator::{PaginationParams, paginate};
use crate::error::CliError;
use crate::output::{OutputFormat, print_list, print_output};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum UsersAction {
    /// List all users
    List {
        #[command(flatten)]
        pagination: PaginationArgs,
    },
    /// Get a user by ID
    Get {
        /// User ID
        id: String,
    },
    /// Get the current bot user
    Me,
}

impl UsersAction {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::List { pagination } => {
                let users = paginate(
                    &PaginationParams {
                        page_size: pagination.page_size,
                        start_cursor: pagination.start_cursor.clone(),
                        fetch_all: pagination.all,
                        limit: pagination.limit,
                    },
                    |cursor, size| {
                        let client = client.clone();
                        async move { client.list_users(cursor, size).await }
                    },
                )
                .await?;
                print_list(&users, format)
            }
            Self::Get { id } => {
                let user = client.get_user(id).await?;
                print_output(&user, format)
            }
            Self::Me => {
                let user = client.get_me().await?;
                print_output(&user, format)
            }
        }
    }
}
