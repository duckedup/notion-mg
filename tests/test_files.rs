mod common;

use common::{json_response, setup_mock_server};
use notion_mg::api::files::{FileBlockKind, MULTI_PART_CHUNK_BYTES, SINGLE_PART_MAX_BYTES};
use serde_json::{Value, json};
use std::io::Write;
use wiremock::matchers::{body_partial_json, body_string_contains, header_regex, method, path};
use wiremock::{Mock, MockServer};

fn upload(id: &str, status: &str, content_type: &str) -> Value {
    json!({
        "object": "file_upload",
        "id": id,
        "created_time": "2026-01-01T00:00:00.000Z",
        "created_by": { "id": "bot-1", "type": "bot" },
        "status": status,
        "filename": "notes.txt",
        "content_type": content_type,
    })
}

fn appended_block(block_type: &str) -> Value {
    json!({
        "results": [{
            "object": "block",
            "id": "block-1",
            "created_time": "2026-01-01T00:00:00.000Z",
            "last_edited_time": "2026-01-01T00:00:00.000Z",
            "type": block_type,
            "has_children": false,
            block_type: { "type": "file", "file": { "url": "https://example.com/f" } },
        }],
        "has_more": false,
        "next_cursor": null,
    })
}

fn temp_file(name: &str, bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::File::create(&path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
    (dir, path)
}

async fn mock_single_part(server: &MockServer, content_type: &str) {
    Mock::given(method("POST"))
        .and(path("/file_uploads"))
        .and(body_partial_json(
            json!({ "mode": "single_part", "content_type": content_type }),
        ))
        .respond_with(json_response(upload("fu-1", "pending", content_type)))
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/file_uploads/fu-1/send"))
        .and(header_regex(
            "content-type",
            "^multipart/form-data; boundary=",
        ))
        .respond_with(json_response(upload("fu-1", "uploaded", content_type)))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn single_part_upload_sends_file_as_multipart_form() {
    let (server, client) = setup_mock_server().await;
    let (_dir, file) = temp_file("notes.txt", b"hello notion");

    Mock::given(method("POST"))
        .and(path("/file_uploads"))
        .and(body_partial_json(json!({
            "mode": "single_part",
            "filename": "notes.txt",
            "content_type": "text/plain",
        })))
        .respond_with(json_response(upload("fu-1", "pending", "text/plain")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/file_uploads/fu-1/send"))
        .and(header_regex(
            "content-type",
            "^multipart/form-data; boundary=",
        ))
        .and(body_string_contains("filename=\"notes.txt\""))
        .and(body_string_contains("hello notion"))
        .respond_with(json_response(upload("fu-1", "uploaded", "text/plain")))
        .expect(1)
        .mount(&server)
        .await;

    let result = client.upload_file(&file, None).await.unwrap();
    assert_eq!(result.id, "fu-1");
    assert_eq!(result.status, "uploaded");
}

#[tokio::test]
async fn large_file_is_split_into_parts_and_completed() {
    let (server, client) = setup_mock_server().await;
    let size = SINGLE_PART_MAX_BYTES + 5 * 1024 * 1024;
    let parts = size.div_ceil(MULTI_PART_CHUNK_BYTES);
    let (_dir, file) = temp_file("big.bin", &vec![0u8; size as usize]);

    Mock::given(method("POST"))
        .and(path("/file_uploads"))
        .and(body_partial_json(
            json!({ "mode": "multi_part", "number_of_parts": parts }),
        ))
        .respond_with(json_response(upload(
            "fu-big",
            "pending",
            "application/octet-stream",
        )))
        .expect(1)
        .mount(&server)
        .await;
    for n in 1..=parts {
        Mock::given(method("POST"))
            .and(path("/file_uploads/fu-big/send"))
            .and(body_string_contains(format!(
                "name=\"part_number\"\r\n\r\n{n}\r\n"
            )))
            .respond_with(json_response(upload(
                "fu-big",
                "pending",
                "application/octet-stream",
            )))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("POST"))
        .and(path("/file_uploads/fu-big/complete"))
        .respond_with(json_response(upload(
            "fu-big",
            "uploaded",
            "application/octet-stream",
        )))
        .expect(1)
        .mount(&server)
        .await;

    let result = client.upload_file(&file, None).await.unwrap();
    assert_eq!(result.status, "uploaded");
}

#[tokio::test]
async fn attach_appends_block_typed_by_content_type() {
    let (server, client) = setup_mock_server().await;
    let (_dir, file) = temp_file("diagram.png", b"\x89PNG");
    mock_single_part(&server, "image/png").await;

    Mock::given(method("PATCH"))
        .and(path("/blocks/page-1/children"))
        .and(body_partial_json(json!({
            "children": [{
                "type": "image",
                "image": {
                    "type": "file_upload",
                    "file_upload": { "id": "fu-1" },
                    "caption": [{ "type": "text", "text": { "content": "Architecture" } }],
                },
            }],
        })))
        .respond_with(json_response(appended_block("image")))
        .expect(1)
        .mount(&server)
        .await;

    let blocks = client
        .attach_file("page-1", &file, None, None, Some("Architecture"), None)
        .await
        .unwrap();
    assert_eq!(blocks[0].block_type, "image");
}

#[tokio::test]
async fn attach_honours_explicit_block_kind() {
    let (server, client) = setup_mock_server().await;
    let (_dir, file) = temp_file("diagram.png", b"\x89PNG");
    mock_single_part(&server, "image/png").await;

    Mock::given(method("PATCH"))
        .and(path("/blocks/page-1/children"))
        .and(body_partial_json(json!({
            "children": [{ "type": "file", "file": { "file_upload": { "id": "fu-1" } } }],
        })))
        .respond_with(json_response(appended_block("file")))
        .expect(1)
        .mount(&server)
        .await;

    client
        .attach_file("page-1", &file, None, Some(FileBlockKind::File), None, None)
        .await
        .unwrap();
}

#[test]
fn block_kind_follows_content_type() {
    assert_eq!(
        FileBlockKind::from_content_type("image/png"),
        FileBlockKind::Image
    );
    assert_eq!(
        FileBlockKind::from_content_type("video/mp4"),
        FileBlockKind::Video
    );
    assert_eq!(
        FileBlockKind::from_content_type("audio/mpeg"),
        FileBlockKind::Audio
    );
    assert_eq!(
        FileBlockKind::from_content_type("application/pdf"),
        FileBlockKind::Pdf
    );
    assert_eq!(
        FileBlockKind::from_content_type("text/csv"),
        FileBlockKind::File
    );
}

#[test]
fn file_block_text_is_its_caption() {
    let block: notion_mg::api::blocks::Block = serde_json::from_value(json!({
        "object": "block",
        "id": "block-1",
        "created_time": "2026-01-01T00:00:00.000Z",
        "last_edited_time": "2026-01-01T00:00:00.000Z",
        "type": "image",
        "image": {
            "type": "file",
            "file": { "url": "https://example.com/f.png" },
            "caption": [{ "type": "text", "plain_text": "Architecture", "href": null }],
        },
    }))
    .unwrap();
    assert_eq!(block.plain_text(), "Architecture");
}
