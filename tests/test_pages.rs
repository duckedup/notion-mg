mod common;

use common::{json_response, setup_mock_server};
use serde_json::json;
use wiremock::Mock;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_page() {
    let (server, client) = setup_mock_server().await;
    let page_id = "b55c9c91-384d-452b-81db-d1ef79372b75";

    Mock::given(method("GET"))
        .and(path(format!("/pages/{page_id}")))
        .respond_with(json_response(json!({
            "object": "page",
            "id": page_id,
            "created_time": "2023-01-01T00:00:00.000Z",
            "last_edited_time": "2023-06-15T12:00:00.000Z",
            "parent": { "type": "workspace", "workspace": true },
            "archived": false,
            "in_trash": false,
            "properties": {
                "title": {
                    "id": "title",
                    "type": "title",
                    "title": [
                        {
                            "type": "text",
                            "plain_text": "Test Page",
                            "text": { "content": "Test Page", "link": null },
                            "href": null
                        }
                    ]
                }
            },
            "url": "https://www.notion.so/Test-Page-b55c9c91384d452b81dbd1ef79372b75"
        })))
        .mount(&server)
        .await;

    let page = client.get_page(page_id).await.unwrap();
    assert_eq!(page.id, page_id);
    assert_eq!(page.title(), "Test Page");
    assert!(!page.archived);
}

#[tokio::test]
async fn test_search_pages() {
    let (server, client) = setup_mock_server().await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(json_response(json!({
            "object": "list",
            "results": [
                {
                    "object": "page",
                    "id": "page-1",
                    "created_time": "2023-01-01T00:00:00.000Z",
                    "last_edited_time": "2023-06-15T12:00:00.000Z",
                    "parent": { "type": "workspace", "workspace": true },
                    "archived": false,
                    "in_trash": false,
                    "properties": {
                        "title": {
                            "type": "title",
                            "title": [{ "type": "text", "plain_text": "Found Page", "href": null }]
                        }
                    }
                }
            ],
            "has_more": false,
            "next_cursor": null,
            "type": "page_or_database"
        })))
        .mount(&server)
        .await;

    let response = client
        .search_pages(Some("Found"), None, None)
        .await
        .unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].title(), "Found Page");
}
