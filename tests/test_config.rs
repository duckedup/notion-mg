use notion_mg::config::Config;
use notion_mg::config::auth::resolve_api_key;

#[test]
fn test_cli_key_takes_precedence() {
    let config = Config {
        api_key: Some("config-key".to_string()),
    };
    let result = resolve_api_key(Some("cli-key"), &config).unwrap();
    assert_eq!(result, "cli-key");
}

#[test]
fn test_config_key_used_as_fallback() {
    let config = Config {
        api_key: Some("config-key".to_string()),
    };
    unsafe { std::env::remove_var("NOTION_API_KEY") };
    let result = resolve_api_key(None, &config).unwrap();
    assert_eq!(result, "config-key");
}

#[test]
fn test_no_key_returns_error() {
    let config = Config { api_key: None };
    unsafe { std::env::remove_var("NOTION_API_KEY") };
    let result = resolve_api_key(None, &config);
    assert!(result.is_err());
}

#[test]
fn test_config_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = Config {
        api_key: Some("test-key".to_string()),
    };
    let contents = toml::to_string_pretty(&config).unwrap();
    std::fs::write(&config_path, &contents).unwrap();

    let loaded: Config = toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(loaded.api_key.as_deref(), Some("test-key"));
}
