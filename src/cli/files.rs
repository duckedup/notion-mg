use crate::api::files::FileBlockKind;
use crate::cli::common::PaginationArgs;
use crate::client::NotionClient;
use crate::client::paginator::{PaginationParams, paginate};
use crate::error::CliError;
use crate::output::{OutputFormat, print_list, print_output};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum FilesAction {
    /// Upload a local file to Notion and attach it to a page or block in one step
    Attach {
        /// Local file to upload
        path: PathBuf,

        /// Page or block ID to append the file to
        #[arg(long)]
        to: String,

        /// Block type to create (inferred from the file type by default)
        #[arg(long = "as", value_enum)]
        kind: Option<FileBlockKind>,

        /// Caption shown under the file
        #[arg(long)]
        caption: Option<String>,

        /// Insert after this block ID instead of at the end
        #[arg(long)]
        after: Option<String>,

        /// MIME type (guessed from the file extension by default)
        #[arg(long)]
        content_type: Option<String>,
    },
    /// Upload a local file without attaching it; reference the returned ID within an hour
    Upload {
        /// Local file to upload
        path: PathBuf,

        /// MIME type (guessed from the file extension by default)
        #[arg(long)]
        content_type: Option<String>,
    },
    /// Get a file upload by ID
    Get {
        /// File upload ID
        id: String,
    },
    /// List file uploads
    List {
        /// Filter by status: pending, uploaded, expired, or failed
        #[arg(long)]
        status: Option<String>,

        #[command(flatten)]
        pagination: PaginationArgs,
    },
}

impl FilesAction {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::Attach {
                path,
                to,
                kind,
                caption,
                after,
                content_type,
            } => {
                let blocks = client
                    .attach_file(
                        to,
                        path,
                        content_type.as_deref(),
                        *kind,
                        caption.as_deref(),
                        after.as_deref(),
                    )
                    .await?;
                print_list(&blocks, format)
            }
            Self::Upload { path, content_type } => {
                let upload = client.upload_file(path, content_type.as_deref()).await?;
                print_output(&upload, format)
            }
            Self::Get { id } => {
                let upload = client.get_file_upload(id).await?;
                print_output(&upload, format)
            }
            Self::List { status, pagination } => {
                let uploads = paginate(
                    &PaginationParams {
                        page_size: pagination.page_size,
                        start_cursor: pagination.start_cursor.clone(),
                        fetch_all: pagination.all,
                        limit: pagination.limit,
                    },
                    |cursor, size| {
                        let client = client.clone();
                        let status = status.clone();
                        async move {
                            client
                                .list_file_uploads(status.as_deref(), cursor, size)
                                .await
                        }
                    },
                )
                .await?;
                print_list(&uploads, format)
            }
        }
    }
}
