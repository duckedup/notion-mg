use crate::output::OutputFormat;
use clap::Args;

#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// Notion API key (overrides env var and config file)
    #[arg(long, global = true, env = "NOTION_API_KEY")]
    pub api_key: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Output as pretty-printed JSON
    #[arg(long, global = true)]
    pub json_pretty: bool,

    /// Enable verbose logging
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

impl GlobalArgs {
    pub fn output_format(&self) -> OutputFormat {
        if self.json_pretty {
            OutputFormat::JsonPretty
        } else if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::Pretty
        }
    }
}

#[derive(Debug, Args)]
pub struct PaginationArgs {
    /// Maximum number of results to return
    #[arg(long)]
    pub limit: Option<usize>,

    /// Cursor for pagination
    #[arg(long)]
    pub start_cursor: Option<String>,

    /// Fetch all pages of results
    #[arg(long, default_value = "false")]
    pub all: bool,

    /// Number of results per page (max 100)
    #[arg(long)]
    pub page_size: Option<u32>,
}
