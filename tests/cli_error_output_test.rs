use serde_json::Value;
use std::process::Command;

fn uxc_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_uxc"))
}

fn parse_stdout_json(output: &std::process::Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("stdout should be valid JSON")
}

#[test]
fn protocol_detection_failure_uses_error_envelope() {
    let output = uxc_command()
        .arg("http://127.0.0.1:9")
        .arg("list")
        .output()
        .expect("failed to run uxc");

    assert!(!output.status.success(), "command should fail");

    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "PROTOCOL_DETECTION_FAILED");
    assert!(json["error"]["message"].is_string());
}

#[test]
fn operation_execution_failure_uses_error_envelope() {
    let mut server = mockito::Server::new();
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

    let output = uxc_command()
        .arg(server.url())
        .arg("bad-format")
        .output()
        .expect("failed to run uxc");

    assert!(!output.status.success(), "command should fail");

    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(json["error"]["message"].is_string());
}

#[test]
fn openapi_legacy_operation_format_is_rejected() {
    let mut server = mockito::Server::new();
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

    let output = uxc_command()
        .arg(server.url())
        .arg("GET /pets")
        .output()
        .expect("failed to run uxc");

    assert!(!output.status.success(), "command should fail");

    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("method:/path"),
        "unexpected error message: {}",
        json["error"]["message"]
    );
}

#[test]
fn generic_jsonrpc_endpoint_is_not_misdetected_as_mcp() {
    let mut server = mockito::Server::new();
    let _jsonrpc_error = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#,
        )
        .create();

    let output = uxc_command()
        .arg(server.url())
        .arg("list")
        .output()
        .expect("failed to run uxc");

    assert!(!output.status.success(), "command should fail");

    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "PROTOCOL_DETECTION_FAILED");
}
