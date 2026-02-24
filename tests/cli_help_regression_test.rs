use std::process::Command;

fn uxc_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_uxc"))
}

#[test]
fn global_help_flag_works() {
    let output = uxc_command().arg("-h").output().expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Universal X-Protocol Call"));
    assert!(stdout.contains("describe"));
}

#[test]
fn operation_help_works_with_dynamic_syntax() {
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
        .arg("help")
        .output()
        .expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(json["ok"], true);
    assert_eq!(json["kind"], "operation_detail");
    assert_eq!(json["operation"], "GET /pets");
}

#[test]
fn text_and_format_flags_are_mutually_exclusive() {
    let output = uxc_command()
        .arg("--format")
        .arg("json")
        .arg("--text")
        .output()
        .expect("failed to run uxc");

    assert!(!output.status.success(), "command should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"), "stderr: {}", stderr);
}

#[test]
fn exec_subcommand_executes_operation() {
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
    let _call = server
        .mock("GET", "/pets")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"items":[]}"#)
        .create();

    let output = uxc_command()
        .arg(server.url())
        .arg("exec")
        .arg("GET /pets")
        .output()
        .expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(json["ok"], true);
    assert_eq!(json["kind"], "call_result");
    assert_eq!(json["operation"], "GET /pets");
}
