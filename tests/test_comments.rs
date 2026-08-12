mod common;

use common::{json_response, setup_mock_server};
use notion_mg::client::NotionClient;
use notion_mg::error::CliError;
use serde_json::{Value, json};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn comment(id: &str, text: &str) -> Value {
    json!({
        "object": "comment",
        "id": id,
        "parent": { "type": "page_id", "page_id": "page-1" },
        "discussion_id": format!("discussion-{id}"),
        "created_time": "2026-01-01T00:00:00.000Z",
        "rich_text": [{ "type": "text", "plain_text": text, "href": null }],
    })
}

fn block(id: &str, text: &str, has_children: bool) -> Value {
    block_of_type(id, "paragraph", text, has_children)
}

fn block_of_type(id: &str, block_type: &str, text: &str, has_children: bool) -> Value {
    json!({
        "object": "block",
        "id": id,
        "created_time": "2026-01-01T00:00:00.000Z",
        "last_edited_time": "2026-01-01T00:00:00.000Z",
        "type": block_type,
        "has_children": has_children,
        block_type: { "rich_text": [{ "type": "text", "plain_text": text, "href": null }] },
    })
}

fn page(results: Vec<Value>) -> Value {
    json!({ "results": results, "has_more": false, "next_cursor": null })
}

async fn mock_comments(server: &MockServer, block_id: &str, comments: Vec<Value>) {
    Mock::given(method("GET"))
        .and(path("/comments"))
        .and(query_param("block_id", block_id))
        .respond_with(json_response(page(comments)))
        .mount(server)
        .await;
}

async fn mock_children(server: &MockServer, block_id: &str, blocks: Vec<Value>) {
    Mock::given(method("GET"))
        .and(path(format!("/blocks/{block_id}/children")))
        .respond_with(json_response(page(blocks)))
        .mount(server)
        .await;
}

/// A page with a page-level comment, an inline comment on a top-level block, and
/// another nested one level deeper.
async fn document_fixture() -> (MockServer, NotionClient) {
    let (server, client) = setup_mock_server().await;

    mock_comments(&server, "page-1", vec![comment("c-page", "page level")]).await;
    mock_children(
        &server,
        "page-1",
        vec![
            block("block-a", "first paragraph", false),
            block("block-b", "a toggle", true),
        ],
    )
    .await;

    mock_comments(&server, "block-a", vec![comment("c-a", "inline on a")]).await;
    mock_comments(&server, "block-b", vec![]).await;
    mock_children(&server, "block-b", vec![block("block-c", "nested", false)]).await;
    mock_comments(&server, "block-c", vec![comment("c-c", "inline on c")]).await;

    (server, client)
}

#[tokio::test]
async fn finds_page_level_and_inline_comments() {
    let (_server, client) = document_fixture().await;

    let found = client.fetch_document_comments("page-1", 10).await.unwrap();

    let ids: Vec<&str> = found.iter().map(|c| c.comment.id.as_str()).collect();
    assert_eq!(ids, ["c-page", "c-a", "c-c"], "expected document order");
}

#[tokio::test]
async fn comments_carry_their_anchor() {
    let (_server, client) = document_fixture().await;

    let found = client.fetch_document_comments("page-1", 10).await.unwrap();

    assert_eq!(found[0].anchor_type, "page");
    assert_eq!(found[0].anchor_id, "page-1");
    assert_eq!(found[0].anchor_text, "");

    assert_eq!(found[1].anchor_type, "paragraph");
    assert_eq!(found[1].anchor_id, "block-a");
    assert_eq!(found[1].anchor_text, "first paragraph");

    assert_eq!(found[2].anchor_id, "block-c");
    assert_eq!(found[2].anchor_text, "nested");
}

#[tokio::test]
async fn max_depth_zero_returns_only_page_level_comments() {
    let (_server, client) = document_fixture().await;

    let found = client.fetch_document_comments("page-1", 0).await.unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].comment.id, "c-page");
}

#[tokio::test]
async fn max_depth_stops_the_descent() {
    let (_server, client) = document_fixture().await;

    let found = client.fetch_document_comments("page-1", 1).await.unwrap();

    let ids: Vec<&str> = found.iter().map(|c| c.comment.id.as_str()).collect();
    assert_eq!(ids, ["c-page", "c-a"], "block-c sits below the depth limit");
}

#[tokio::test]
async fn child_pages_are_not_walked() {
    let (server, client) = setup_mock_server().await;

    mock_comments(&server, "page-1", vec![]).await;
    mock_children(
        &server,
        "page-1",
        vec![block_of_type("sub-page", "child_page", "Sub page", true)],
    )
    .await;
    mock_comments(&server, "sub-page", vec![comment("c-sub", "on the link")]).await;

    Mock::given(method("GET"))
        .and(path("/blocks/sub-page/children"))
        .respond_with(json_response(page(vec![])))
        .expect(0)
        .mount(&server)
        .await;

    let found = client.fetch_document_comments("page-1", 10).await.unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].comment.id, "c-sub");
    assert_eq!(found[0].anchor_type, "child_page");

    // Verifies the expect(0) above: the sub-page's own blocks are a separate document.
    server.verify().await;
}

#[tokio::test]
async fn paginated_comments_are_all_collected() {
    let (server, client) = setup_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/comments"))
        .and(query_param("block_id", "page-1"))
        .and(query_param("start_cursor", "cursor-2"))
        .respond_with(json_response(page(vec![comment("c-2", "second page")])))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/comments"))
        .and(query_param("block_id", "page-1"))
        .respond_with(json_response(json!({
            "results": [comment("c-1", "first page")],
            "has_more": true,
            "next_cursor": "cursor-2",
        })))
        .mount(&server)
        .await;

    mock_children(&server, "page-1", vec![]).await;

    let found = client.fetch_document_comments("page-1", 10).await.unwrap();

    let ids: Vec<&str> = found.iter().map(|c| c.comment.id.as_str()).collect();
    assert_eq!(ids, ["c-1", "c-2"]);
}

#[tokio::test]
async fn a_block_deleted_mid_walk_is_skipped() {
    let (server, client) = setup_mock_server().await;

    mock_comments(&server, "page-1", vec![comment("c-page", "page level")]).await;
    mock_children(
        &server,
        "page-1",
        vec![
            block("gone", "deleted", false),
            block("kept", "here", false),
        ],
    )
    .await;
    mock_comments(&server, "kept", vec![comment("c-kept", "still here")]).await;

    Mock::given(method("GET"))
        .and(path("/comments"))
        .and(query_param("block_id", "gone"))
        .respond_with(ResponseTemplate::new(404).set_body_string("block not found"))
        .mount(&server)
        .await;

    let found = client.fetch_document_comments("page-1", 10).await.unwrap();

    let ids: Vec<&str> = found.iter().map(|c| c.comment.id.as_str()).collect();
    assert_eq!(ids, ["c-page", "c-kept"]);
}

#[tokio::test]
async fn missing_read_comments_capability_is_explained() {
    let (server, client) = setup_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/comments"))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_string("Insufficient permissions for this endpoint"),
        )
        .mount(&server)
        .await;

    let err = client
        .fetch_document_comments("page-1", 10)
        .await
        .unwrap_err();

    assert!(matches!(err, CliError::Forbidden(_)), "got {err:?}");
    assert_eq!(err.error_type(), "forbidden");

    let message = err.to_string();
    assert!(message.contains("Insufficient permissions"), "{message}");
    assert!(message.contains("Read comments"), "{message}");
}

#[tokio::test]
async fn list_all_comments_covers_a_single_block() {
    let (server, client) = setup_mock_server().await;

    mock_comments(&server, "block-a", vec![comment("c-a", "one")]).await;

    let comments = client.list_all_comments("block-a").await.unwrap();

    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].plain_text(), "one");
}
