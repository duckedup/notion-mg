use notion_mg::client::NotionClient;
use wiremock::MockServer;

pub async fn setup_mock_server() -> (MockServer, NotionClient) {
    let server = MockServer::start().await;
    let client = NotionClient::with_base_url("test-api-key", &server.uri()).unwrap();
    (server, client)
}

pub fn json_response(body: serde_json::Value) -> wiremock::ResponseTemplate {
    wiremock::ResponseTemplate::new(200)
        .set_body_json(body)
        .insert_header("content-type", "application/json")
}
