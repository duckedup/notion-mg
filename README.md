# notion-mg

A CLI tool and SDK for the [Notion API](https://developers.notion.com/), designed for AI agent consumption.

Both a standalone CLI binary and a Rust library — use it from the command line or import it as a crate.

## Install

### From crates.io

```bash
cargo install notion-mg
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/duckedup/notion-mg/releases), or use `cargo-binstall`:

```bash
cargo binstall notion-mg
```

## Authentication

Get a Notion integration token from [notion.so/profile/integrations](https://www.notion.so/profile/integrations).

API key is resolved in order: `--api-key` flag > `NOTION_API_KEY` env var > config file.

Set `NOTION_MG_CONFIG_DIR` to read and write the config somewhere other than the OS
config directory — useful for CI, sandboxes, and keeping separate workspaces apart.

```bash
# Store in config
notion-mg auth init --token ntn_xxx

# Or use env var
export NOTION_API_KEY=ntn_xxx

# Check status
notion-mg auth status
```

## Usage

### Output formats

All commands support three output formats:

- **Pretty** (default) — human-readable tables and detail views
- `--json` — compact JSON for piping
- `--json-pretty` — formatted JSON

### Pages

```bash
# Get a page
notion-mg pages get <page-id>

# Search for pages
notion-mg pages search --query "meeting notes"

# Create a page under another page
notion-mg pages create --parent-id <page-id> --title "New Page"

# Create a page in a database
notion-mg pages create --parent-id <db-id> --parent-type database --title "New Entry"

# Create a page with an icon
notion-mg pages create --parent-id <page-id> --title "Rate Tables" --icon 📐

# Update a page
notion-mg pages update <page-id> --properties '{"Status": {"select": {"name": "Done"}}}'

# Change a page's icon
notion-mg pages update <page-id> --icon 🚦

# Archive a page
notion-mg pages archive <page-id>
```

`--icon` takes an emoji, or an `http(s)` URL for a hosted image.

### Databases

```bash
# Get database schema
notion-mg databases get <database-id>

# Query a database
notion-mg databases query <database-id>

# Query with filter
notion-mg databases query <database-id> --filter '{"property": "Status", "select": {"equals": "In Progress"}}'

# Query with sort
notion-mg databases query <database-id> --sorts '[{"property": "Created", "direction": "descending"}]'

# Search for databases
notion-mg databases search --query "tasks"
```

### Blocks

```bash
# Get a block
notion-mg blocks get <block-id>

# List children of a page or block
notion-mg blocks children <page-id>

# Append content to a page
notion-mg blocks append <page-id> --children '[{"type": "paragraph", "paragraph": {"rich_text": [{"type": "text", "text": {"content": "Hello world"}}]}}]'

# Delete a block
notion-mg blocks delete <block-id>
```

### Markdown

Convert a Markdown file into Notion blocks, without hand-writing block JSON.

```bash
# Print the blocks as JSON ("-" reads stdin)
notion-mg markdown convert --file notes.md

# Append the blocks to an existing page
notion-mg markdown append --file notes.md --block-id <page-id>

# Create a new child page from the file
notion-mg markdown create --file notes.md --parent-page-id <page-id>

# ...with a title and an icon of your own
notion-mg markdown create --file spec.md --parent-page-id <page-id> --title "ENG-3210" --icon 🚦
```

`create` takes its title from `--title`, falling back to the file's first heading and
then the file name, and its icon from `--icon`.

Supported: headings, paragraphs, bulleted/numbered lists with nesting, GFM task lists,
fenced code with language detection, block quotes, dividers, GFM tables, standalone
images, and inline bold/italic/strikethrough/code/links. Anything else becomes a
paragraph. Documents longer than Notion's 100-block request limit are appended in
sequential batches.

### Users

```bash
# List all users
notion-mg users list

# Get current bot user
notion-mg users me

# Get a specific user
notion-mg users get <user-id>
```

### Comments

```bash
# Every comment on a document — page-level plus inline comments on each block
notion-mg comments document --page-id <page-id>

# List comments on one specific page or block
notion-mg comments list --block-id <block-id>

# Add a comment
notion-mg comments create --page-id <page-id> --text "Looks good!"
```

Notion only exposes comments per block, so `comments document` walks the block tree and
queries each node — roughly one request per block, against a ~3 req/s rate limit. Use
`--max-depth` to bound the descent (`0` fetches page-level comments only). Child pages
and databases are skipped: they are separate documents.

Each result carries the block it is anchored to (`anchor_id`, `anchor_type`,
`anchor_text`), so an inline comment can be located in the document.

The Notion API returns only **unresolved** comments; resolved threads are not
retrievable. Reading comments also requires the "Read comments" capability on your
integration, at [notion.so/profile/integrations](https://www.notion.so/profile/integrations).

### Search

```bash
# Search everything
notion-mg search --query "project plan"

# Search only pages
notion-mg search --query "meeting" --object-type page

# Search only databases
notion-mg search --object-type database

# Raw search with full JSON body
notion-mg raw-search '{"query": "test", "page_size": 5}'
```

### Pagination

List commands support pagination:

```bash
# Limit results
notion-mg pages search --query "notes" --limit 10

# Fetch all pages
notion-mg databases query <database-id> --all

# Manual cursor pagination
notion-mg users list --page-size 10 --start-cursor <cursor>
```

## SDK Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
notion-mg = "0.3"
```

```rust
use notion_mg::client::NotionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = NotionClient::new("ntn_xxx")?;

    // Get current user
    let me = client.get_me().await?;
    println!("Bot: {:?}", me.name);

    // Search pages
    let results = client.search_pages(Some("meeting"), None, None).await?;
    for page in results.results {
        println!("{}: {}", page.id, page.title());
    }

    // Query a database
    let entries = client.query_database("db-id", None, None, None, None).await?;
    for entry in entries.results {
        println!("{}", entry.title());
    }

    // Get page content
    let blocks = client.get_block_children("page-id", None, None).await?;
    for block in blocks.results {
        println!("[{}] {}", block.block_type, block.plain_text());
    }

    Ok(())
}
```

## Error Handling

Errors output structured JSON to stderr with distinct exit codes:

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Authentication error |
| 3 | Rate limited |
| 4 | Not found |
| 5 | Invalid input |
| 6 | API error |
| 7 | Forbidden (integration lacks a required capability) |

## Development

Requires [just](https://github.com/casey/just) as a task runner.

```bash
just check    # fmt + clippy + test
just fmt      # format code
just lint     # run clippy
just test     # run tests
```

## License

MIT
