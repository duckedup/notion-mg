pub mod pretty;

use crate::error::CliError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    JsonPretty,
}

pub fn print_output<T: Serialize + pretty::PrettyPrint>(
    data: &T,
    format: OutputFormat,
) -> Result<(), CliError> {
    match format {
        OutputFormat::Pretty => {
            data.pretty();
            Ok(())
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(data)?);
            Ok(())
        }
        OutputFormat::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(data)?);
            Ok(())
        }
    }
}

pub fn print_list<T: Serialize + pretty::PrettyPrint>(
    data: &[T],
    format: OutputFormat,
) -> Result<(), CliError> {
    match format {
        OutputFormat::Pretty => {
            if data.is_empty() {
                println!("No results found.");
                return Ok(());
            }
            let rows: Vec<Vec<String>> = data.iter().map(|item| item.pretty_row()).collect();
            let headers = T::headers();
            pretty::print_table(&headers, &rows);
            Ok(())
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(data)?);
            Ok(())
        }
        OutputFormat::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(data)?);
            Ok(())
        }
    }
}
