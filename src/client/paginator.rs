use crate::api::common::PaginatedResponse;
use crate::error::CliError;
use serde::de::DeserializeOwned;
use std::future::Future;

pub struct PaginationParams {
    pub page_size: Option<u32>,
    pub start_cursor: Option<String>,
    pub fetch_all: bool,
    pub limit: Option<usize>,
}

pub async fn paginate<T, F, Fut>(params: &PaginationParams, fetch: F) -> Result<Vec<T>, CliError>
where
    T: DeserializeOwned,
    F: Fn(Option<String>, Option<u32>) -> Fut,
    Fut: Future<Output = Result<PaginatedResponse<T>, CliError>>,
{
    let mut all_results = Vec::new();
    let mut cursor = params.start_cursor.clone();
    let limit = params.limit.unwrap_or(usize::MAX);

    loop {
        let response = fetch(cursor, params.page_size).await?;
        all_results.extend(response.results);

        if all_results.len() >= limit {
            all_results.truncate(limit);
            break;
        }

        if !response.has_more || !params.fetch_all {
            break;
        }

        cursor = response.next_cursor;
    }

    Ok(all_results)
}
