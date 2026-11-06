use crate::api::blocks::Block;
use crate::api::common::{PaginatedResponse, make_rich_text};
use crate::client::NotionClient;
use crate::error::CliError;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use tokio::io::AsyncReadExt;

/// Notion rejects single-part uploads above this size; larger files must go multi-part.
pub const SINGLE_PART_MAX_BYTES: u64 = 20 * 1024 * 1024;
/// Notion accepts 5–20 MiB per part (the last may be smaller) and recommends 10 MiB.
pub const MULTI_PART_CHUNK_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpload {
    pub object: String,
    pub id: String,
    pub created_time: String,
    #[serde(default)]
    pub last_edited_time: Option<String>,
    // Unlike pages, this is `{id, type}` with no `object` field, so `PartialUser` won't parse it.
    #[serde(default)]
    pub created_by: Option<Value>,
    #[serde(default)]
    pub expiry_time: Option<String>,
    pub status: String,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_length: Option<u64>,
    #[serde(default)]
    pub upload_url: Option<String>,
    #[serde(default)]
    pub complete_url: Option<String>,
    #[serde(default)]
    pub number_of_parts: Option<Value>,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FileBlockKind {
    File,
    Image,
    Pdf,
    Video,
    Audio,
}

impl FileBlockKind {
    pub fn from_content_type(content_type: &str) -> Self {
        match content_type.split('/').next() {
            Some("image") => Self::Image,
            Some("video") => Self::Video,
            Some("audio") => Self::Audio,
            _ if content_type == "application/pdf" => Self::Pdf,
            _ => Self::File,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Image => "image",
            Self::Pdf => "pdf",
            Self::Video => "video",
            Self::Audio => "audio",
        }
    }
}

pub fn guess_content_type(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string()
}

/// A block that embeds an uploaded file. The upload must be in `uploaded` status.
pub fn file_block(kind: FileBlockKind, file_upload_id: &str, caption: Option<&str>) -> Value {
    let mut body = json!({
        "type": "file_upload",
        "file_upload": { "id": file_upload_id },
    });
    if let Some(caption) = caption {
        body["caption"] = json!(make_rich_text(caption));
    }
    json!({ "type": kind.as_str(), kind.as_str(): body })
}

impl NotionClient {
    pub async fn create_file_upload(
        &self,
        filename: &str,
        content_type: &str,
        number_of_parts: Option<u64>,
    ) -> Result<FileUpload, CliError> {
        let mut body = json!({
            "mode": if number_of_parts.is_some() { "multi_part" } else { "single_part" },
            "filename": filename,
            "content_type": content_type,
        });
        if let Some(parts) = number_of_parts {
            body["number_of_parts"] = json!(parts);
        }
        self.post("/file_uploads", &body).await
    }

    /// `part_number` is 1-based and required only for multi-part uploads.
    pub async fn send_file_upload(
        &self,
        file_upload_id: &str,
        filename: &str,
        content_type: &str,
        bytes: Vec<u8>,
        part_number: Option<u64>,
    ) -> Result<FileUpload, CliError> {
        let part = Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str(content_type)
            .map_err(|e| CliError::InvalidInput(format!("Invalid content type: {e}")))?;
        let mut form = Form::new().part("file", part);
        if let Some(n) = part_number {
            form = form.text("part_number", n.to_string());
        }
        self.post_multipart(&format!("/file_uploads/{file_upload_id}/send"), form)
            .await
    }

    pub async fn complete_file_upload(&self, file_upload_id: &str) -> Result<FileUpload, CliError> {
        self.post(
            &format!("/file_uploads/{file_upload_id}/complete"),
            &json!({}),
        )
        .await
    }

    pub async fn get_file_upload(&self, file_upload_id: &str) -> Result<FileUpload, CliError> {
        self.get(&format!("/file_uploads/{file_upload_id}")).await
    }

    pub async fn list_file_uploads(
        &self,
        status: Option<&str>,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<PaginatedResponse<FileUpload>, CliError> {
        let mut params = Vec::new();
        if let Some(status) = status {
            params.push(format!("status={status}"));
        }
        if let Some(cursor) = start_cursor {
            params.push(format!("start_cursor={cursor}"));
        }
        if let Some(size) = page_size {
            params.push(format!("page_size={size}"));
        }
        let query = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };
        self.get(&format!("/file_uploads{query}")).await
    }

    /// Uploads a local file, switching to multi-part above [`SINGLE_PART_MAX_BYTES`].
    /// The returned upload must be attached within an hour or Notion expires it.
    pub async fn upload_file(
        &self,
        path: &Path,
        content_type: Option<&str>,
    ) -> Result<FileUpload, CliError> {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CliError::InvalidInput(format!("Not a file: {}", path.display())))?
            .to_string();
        let content_type = content_type
            .map(str::to_string)
            .unwrap_or_else(|| guess_content_type(path));
        let size = tokio::fs::metadata(path).await?.len();

        if size <= SINGLE_PART_MAX_BYTES {
            let upload = self
                .create_file_upload(&filename, &content_type, None)
                .await?;
            let bytes = tokio::fs::read(path).await?;
            return self
                .send_file_upload(&upload.id, &filename, &content_type, bytes, None)
                .await;
        }

        let parts = size.div_ceil(MULTI_PART_CHUNK_BYTES);
        let upload = self
            .create_file_upload(&filename, &content_type, Some(parts))
            .await?;
        let mut file = tokio::fs::File::open(path).await?;
        for part_number in 1..=parts {
            let mut chunk = Vec::with_capacity(MULTI_PART_CHUNK_BYTES as usize);
            (&mut file)
                .take(MULTI_PART_CHUNK_BYTES)
                .read_to_end(&mut chunk)
                .await?;
            self.send_file_upload(
                &upload.id,
                &filename,
                &content_type,
                chunk,
                Some(part_number),
            )
            .await?;
        }
        self.complete_file_upload(&upload.id).await
    }

    /// Uploads a local file and appends it to a page or block. The block type is inferred
    /// from the content type unless `kind` is given.
    pub async fn attach_file(
        &self,
        parent_id: &str,
        path: &Path,
        content_type: Option<&str>,
        kind: Option<FileBlockKind>,
        caption: Option<&str>,
        after: Option<&str>,
    ) -> Result<Vec<Block>, CliError> {
        let content_type = content_type
            .map(str::to_string)
            .unwrap_or_else(|| guess_content_type(path));
        let upload = self.upload_file(path, Some(&content_type)).await?;
        let kind = kind.unwrap_or_else(|| FileBlockKind::from_content_type(&content_type));
        let block = file_block(kind, &upload.id, caption);
        let response = self
            .append_block_children(parent_id, vec![block], after)
            .await?;
        Ok(response.results)
    }
}
