use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    Command::cargo_bin("notion-mg")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Notion API"));
}

#[test]
fn test_version() {
    Command::cargo_bin("notion-mg")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_no_command() {
    Command::cargo_bin("notion-mg").unwrap().assert().failure();
}

#[test]
fn test_pages_help() {
    Command::cargo_bin("notion-mg")
        .unwrap()
        .args(["pages", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Work with pages"));
}

#[test]
fn test_databases_help() {
    Command::cargo_bin("notion-mg")
        .unwrap()
        .args(["databases", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Work with databases"));
}

#[test]
fn test_blocks_help() {
    Command::cargo_bin("notion-mg")
        .unwrap()
        .args(["blocks", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Work with blocks"));
}

#[test]
fn test_auth_help() {
    Command::cargo_bin("notion-mg")
        .unwrap()
        .args(["auth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage authentication"));
}

#[test]
fn test_missing_api_key() {
    // Without an empty config dir the CLI falls back to the developer's real
    // config.toml, reaches the live API, and the assertion below never fires.
    let config_dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("notion-mg")
        .unwrap()
        .env_remove("NOTION_API_KEY")
        .env("NOTION_MG_CONFIG_DIR", config_dir.path())
        .args(["users", "me"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("auth_error"));
}

#[test]
fn test_config_dir_override_is_read() {
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        config_dir.path().join("config.toml"),
        "api_key = \"from-the-override\"\n",
    )
    .unwrap();

    Command::cargo_bin("notion-mg")
        .unwrap()
        .env_remove("NOTION_API_KEY")
        .env("NOTION_MG_CONFIG_DIR", config_dir.path())
        .args(["auth", "status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"config_key_set\":true"))
        .stdout(predicate::str::contains(
            config_dir.path().to_str().unwrap(),
        ));
}

#[test]
fn test_empty_config_dir_reports_no_key() {
    let config_dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("notion-mg")
        .unwrap()
        .env_remove("NOTION_API_KEY")
        .env("NOTION_MG_CONFIG_DIR", config_dir.path())
        .args(["auth", "status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"config_key_set\":false"));
}
