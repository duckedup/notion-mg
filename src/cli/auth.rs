use crate::client::NotionClient;
use crate::config::Config;
use crate::error::CliError;
use crate::output::OutputFormat;
use clap::Subcommand;
use std::io::{self, BufRead, Write};

#[derive(Debug, Subcommand)]
pub enum AuthAction {
    /// Initialize authentication by providing an API key
    Init {
        /// Notion integration token (interactive prompt if omitted)
        #[arg(long)]
        token: Option<String>,
    },
    /// Show current authentication status
    Status,
    /// Remove stored API key
    Revoke,
}

fn prompt_for_token() -> Result<String, CliError> {
    println!("Create a Notion integration at: https://www.notion.so/profile/integrations");
    println!();
    print!("Paste your integration token: ");
    io::stdout()
        .flush()
        .map_err(|e| CliError::General(e.to_string()))?;

    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| CliError::General(e.to_string()))?;

    let token = line.trim().to_string();
    if token.is_empty() {
        return Err(CliError::InvalidInput("No token provided.".to_string()));
    }
    Ok(token)
}

impl AuthAction {
    pub async fn run(&self, format: OutputFormat) -> Result<(), CliError> {
        match self {
            Self::Init { token } => {
                let token = match token {
                    Some(t) => t.clone(),
                    None => prompt_for_token()?,
                };

                let client = NotionClient::new(&token)?;
                let user = client.get_me().await?;

                let mut config = Config::load()?;
                config.api_key = Some(token);
                config.save()?;

                match format {
                    OutputFormat::Pretty => {
                        println!(
                            "Authenticated as: {}",
                            user.name.as_deref().unwrap_or("Unknown")
                        );
                        println!("API key saved to config.");
                    }
                    OutputFormat::Json | OutputFormat::JsonPretty => {
                        let output = serde_json::json!({
                            "status": "authenticated",
                            "user": user,
                        });
                        if matches!(format, OutputFormat::JsonPretty) {
                            println!("{}", serde_json::to_string_pretty(&output)?);
                        } else {
                            println!("{}", serde_json::to_string(&output)?);
                        }
                    }
                }
                Ok(())
            }
            Self::Status => {
                let config = Config::load()?;
                let has_key = config.api_key.is_some();
                let has_env = std::env::var("NOTION_API_KEY").is_ok();

                match format {
                    OutputFormat::Pretty => {
                        println!("Config file: {}", if has_key { "set" } else { "not set" });
                        println!("Environment: {}", if has_env { "set" } else { "not set" });
                        if let Ok(path) = Config::config_path() {
                            println!("Config path: {}", path.display());
                        }
                    }
                    OutputFormat::Json | OutputFormat::JsonPretty => {
                        let output = serde_json::json!({
                            "config_key_set": has_key,
                            "env_key_set": has_env,
                            "config_path": Config::config_path().ok().map(|p| p.display().to_string()),
                        });
                        if matches!(format, OutputFormat::JsonPretty) {
                            println!("{}", serde_json::to_string_pretty(&output)?);
                        } else {
                            println!("{}", serde_json::to_string(&output)?);
                        }
                    }
                }
                Ok(())
            }
            Self::Revoke => {
                let mut config = Config::load()?;
                config.api_key = None;
                config.save()?;

                match format {
                    OutputFormat::Pretty => println!("API key removed from config."),
                    OutputFormat::Json | OutputFormat::JsonPretty => {
                        let output = serde_json::json!({ "status": "revoked" });
                        if matches!(format, OutputFormat::JsonPretty) {
                            println!("{}", serde_json::to_string_pretty(&output)?);
                        } else {
                            println!("{}", serde_json::to_string(&output)?);
                        }
                    }
                }
                Ok(())
            }
        }
    }
}
