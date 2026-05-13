mod common;

use common::{json_response, setup_mock_server};
use serde_json::json;
use wiremock::Mock;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_me() {
    let (server, client) = setup_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/users/me"))
        .respond_with(json_response(json!({
            "object": "user",
            "id": "d40e767c-d7af-4b18-a86d-55c61f1e39a4",
            "type": "bot",
            "name": "Test Bot",
            "avatar_url": null,
            "bot": {}
        })))
        .mount(&server)
        .await;

    let user = client.get_me().await.unwrap();
    assert_eq!(user.name.as_deref(), Some("Test Bot"));
    assert_eq!(user.user_type.as_deref(), Some("bot"));
}

#[tokio::test]
async fn test_list_users() {
    let (server, client) = setup_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(json_response(json!({
            "object": "list",
            "results": [
                {
                    "object": "user",
                    "id": "d40e767c-d7af-4b18-a86d-55c61f1e39a4",
                    "type": "person",
                    "name": "Alice",
                    "avatar_url": null
                },
                {
                    "object": "user",
                    "id": "e79a0b74-3aba-4149-9f74-0bb5791a6ee6",
                    "type": "person",
                    "name": "Bob",
                    "avatar_url": null
                }
            ],
            "has_more": false,
            "next_cursor": null,
            "type": "user"
        })))
        .mount(&server)
        .await;

    let response = client.list_users(None, None).await.unwrap();
    assert_eq!(response.results.len(), 2);
    assert_eq!(response.results[0].name.as_deref(), Some("Alice"));
    assert_eq!(response.results[1].name.as_deref(), Some("Bob"));
}
