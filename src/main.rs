use clap::Parser;
use notion_mg::cli::{Cli, Commands};
use notion_mg::client::NotionClient;
use notion_mg::config::Config;
use notion_mg::config::auth::resolve_api_key;
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.global.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::DEBUG.into()),
            )
            .with_target(false)
            .init();
    }

    let format = cli.global.output_format();

    let result = match cli.command {
        Commands::Auth(action) => action.run(format).await,
        command => {
            let config = match Config::load() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}", e.to_json());
                    process::exit(e.exit_code());
                }
            };

            let api_key = match resolve_api_key(cli.global.api_key.as_deref(), &config) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("{}", e.to_json());
                    process::exit(e.exit_code());
                }
            };

            let client = match NotionClient::new(&api_key) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}", e.to_json());
                    process::exit(e.exit_code());
                }
            };

            match command {
                Commands::Pages(action) => action.run(&client, format).await,
                Commands::Databases(action) => action.run(&client, format).await,
                Commands::Blocks(action) => action.run(&client, format).await,
                Commands::Users(action) => action.run(&client, format).await,
                Commands::Comments(action) => action.run(&client, format).await,
                Commands::Markdown(action) => action.run(&client, format).await,
                Commands::Search(args) => args.run(&client, format).await,
                Commands::RawSearch(args) => args.run(&client, format).await,
                Commands::Auth(_) => unreachable!(),
            }
        }
    };

    if let Err(e) = result {
        eprintln!("{}", e.to_json());
        process::exit(e.exit_code());
    }
}
