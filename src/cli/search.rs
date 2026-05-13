use crate::client::NotionClient;
use crate::error::CliError;
use crate::output::{OutputFormat, print_output};
use clap::Args;
use serde_json::{Value, json};

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query text
    #[arg(long)]
    pub query: Option<String>,

    /// Filter by object type: "page" or "database"
    #[arg(long)]
    pub object_type: Option<String>,

    /// Sort direction: "ascending" or "descending"
    #[arg(long, default_value = "descending")]
    pub sort_direction: String,

    /// Sort by timestamp: "last_edited_time"
    #[arg(long, default_value = "last_edited_time")]
    pub sort_by: String,

    /// Cursor for pagination
    #[arg(long)]
    pub start_cursor: Option<String>,

    /// Number of results per page (max 100)
    #[arg(long)]
    pub page_size: Option<u32>,
}

impl SearchArgs {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        let filter = self.object_type.as_ref().map(|t| {
            json!({
                "value": t,
                "property": "object",
            })
        });

        let sort = Some(json!({
            "direction": self.sort_direction,
            "timestamp": self.sort_by,
        }));

        let results = client
            .search(
                self.query.as_deref(),
                filter,
                sort,
                self.start_cursor.clone(),
                self.page_size,
            )
            .await?;

        match format {
            OutputFormat::Pretty => print_output(&results, format),
            OutputFormat::Json => {
                println!("{}", serde_json::to_string(&results.results)?);
                Ok(())
            }
            OutputFormat::JsonPretty => {
                println!("{}", serde_json::to_string_pretty(&results.results)?);
                Ok(())
            }
        }
    }
}

// Also support a raw JSON search for maximum flexibility
#[derive(Debug, Args)]
pub struct RawSearchArgs {
    /// Full search request body as JSON
    pub body: String,
}

impl RawSearchArgs {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        let body: Value = serde_json::from_str(&self.body)
            .map_err(|e| CliError::InvalidInput(format!("Invalid JSON: {e}")))?;

        let results: Value = client.post("/search", &body).await?;

        match format {
            OutputFormat::Pretty | OutputFormat::JsonPretty => {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string(&results)?);
            }
        }
        Ok(())
    }
}
