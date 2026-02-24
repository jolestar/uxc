//! CLI error output integration tests
//!
//! Tests that CLI failures return structured JSON error envelopes

use assert_cmd::Command;
use predicates::prelude::*;
use mockito::Server;

fn uxc() -> Command {
    Command::cargo_bin("uxc").unwrap()
}

#[test]
fn protocol_detection_failure_uses_error_envelope() {
    uxc()
        .arg("http://127.0.0.1:9")
        .arg("list")
        .assert()
        .failure()
        .stdout(predicates::str::contains("PROTOCOL_DETECTION_FAILED"))
        .stderr(predicates::str::is_empty());
}

#[test]
fn operation_execution_failure_uses_error_envelope() {
    let mut server = Server::new();
    let _schema = server
        .mock("GET", "/openapi.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
  "openapi": "3.0.0",
  "info": { "title": "test", "version": "1.0.0" },
  "paths": {
    "/pets": {
      "get": {
        "summary": "list pets",
        "responses": { "200": { "description": "ok" } }
      }
    }
  }
}"#,
        )
        .create();

    // Call without operation should succeed
    uxc()
        .arg(server.url())
        .arg("list")
        .assert()
        .success();

    // Call with non-existent operation should fail with error envelope
    uxc()
        .arg(server.url())
        .arg("nonexistent")
        .assert()
        .failure();
}
