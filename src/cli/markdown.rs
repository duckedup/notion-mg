use crate::client::NotionClient;
use crate::error::CliError;
use crate::markdown::{chunk_blocks, markdown_to_blocks, parse_inline};
use crate::output::{OutputFormat, print_list, print_output};
use clap::Subcommand;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub enum MarkdownAction {
    /// Convert a Markdown file to Notion blocks and print them as JSON
    Convert {
        /// Markdown file to read ("-" for stdin)
        #[arg(long)]
        file: PathBuf,
    },
    /// Convert a Markdown file and append the blocks to a page or block
    Append {
        /// Markdown file to read ("-" for stdin)
        #[arg(long)]
        file: PathBuf,

        /// Page or block ID to append to
        #[arg(long)]
        block_id: String,
    },
    /// Convert a Markdown file into a new child page
    Create {
        /// Markdown file to read ("-" for stdin)
        #[arg(long)]
        file: PathBuf,

        /// Parent page ID
        #[arg(long)]
        parent_page_id: String,

        /// Page title (defaults to the file's first heading, then the file name)
        #[arg(long)]
        title: Option<String>,
    },
}

impl MarkdownAction {
    pub async fn run(&self, client: &NotionClient, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::Convert { file } => {
                let blocks = markdown_to_blocks(&read_source(file)?);
                print_json(&blocks, format)
            }
            Self::Append { file, block_id } => {
                let blocks = markdown_to_blocks(&read_source(file)?);
                let mut appended = Vec::new();
                // Notion caps each request at 100 blocks, so a long document needs
                // several sequential appends to preserve ordering.
                for chunk in chunk_blocks(blocks) {
                    let result = client.append_block_children(block_id, chunk, None).await?;
                    appended.extend(result.results);
                }
                print_list(&appended, format)
            }
            Self::Create {
                file,
                parent_page_id,
                title,
            } => {
                let source = read_source(file)?;
                let blocks = markdown_to_blocks(&source);
                let title = title
                    .clone()
                    .or_else(|| first_heading(&source))
                    .unwrap_or_else(|| file_stem(file));

                let mut chunks = chunk_blocks(blocks).into_iter();
                let page = client
                    .create_page(
                        serde_json::json!({ "page_id": parent_page_id }),
                        serde_json::json!({
                            "title": { "title": parse_inline(&title) }
                        }),
                        chunks.next(),
                        None,
                        None,
                    )
                    .await?;

                for chunk in chunks {
                    client.append_block_children(&page.id, chunk, None).await?;
                }
                print_output(&page, format)
            }
        }
    }
}

fn read_source(path: &Path) -> Result<String, CliError> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        return Ok(buf);
    }

    std::fs::read_to_string(path)
        .map_err(|e| CliError::InvalidInput(format!("Cannot read {}: {e}", path.display())))
}

fn first_heading(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        let trimmed = line.trim_start();
        trimmed
            .starts_with('#')
            .then(|| trimmed.trim_start_matches('#').trim().to_string())
            .filter(|t| !t.is_empty())
    })
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn print_json(blocks: &[Value], format: OutputFormat) -> Result<(), CliError> {
    let output = match format {
        OutputFormat::Json => serde_json::to_string(blocks)?,
        _ => serde_json::to_string_pretty(blocks)?,
    };
    println!("{output}");
    Ok(())
}
