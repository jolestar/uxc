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
            r##"{
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
}"##,
        )
        .create();

    let output = uxc_command()
        .arg(server.url())
        .arg("--no-cache")
        .arg("get:/pets")
        .arg("help")
        .output()
        .expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(json["ok"], true);
    assert_eq!(json["kind"], "operation_detail");
    assert_eq!(json["operation"], "get:/pets");
    assert_eq!(json["data"]["operation_id"], "get:/pets");
    assert_eq!(json["data"]["display_name"], "GET /pets");
}

#[test]
fn operation_help_includes_openapi_request_body_schema() {
    let mut server = mockito::Server::new();
    let _schema = server
        .mock("GET", "/openapi.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r##"{
  "openapi": "3.0.0",
  "info": { "title": "test", "version": "1.0.0" },
  "paths": {
    "/pet": {
      "post": {
        "summary": "add pet",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": { "$ref": "#/components/schemas/Pet" }
            }
          }
        },
        "responses": { "200": { "description": "ok" } }
      }
    }
  },
  "components": {
    "schemas": {
      "Pet": {
        "type": "object",
        "required": ["name"],
        "properties": {
          "name": { "type": "string" }
        }
      }
    }
  }
}"##,
        )
        .create();

    let output = uxc_command()
        .arg(server.url())
        .arg("--no-cache")
        .arg("post:/pet")
        .arg("help")
        .output()
        .expect("failed to run uxc");

    assert!(
        output.status.success(),
        "command should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(json["kind"], "operation_detail");
    assert_eq!(json["data"]["input_schema"]["kind"], "openapi_request_body");
    assert_eq!(
        json["data"]["input_schema"]["content"]["application/json"]["schema"]["properties"]["name"]
            ["type"],
        "string"
    );
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
fn call_subcommand_executes_operation() {
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
        .arg("--no-cache")
        .arg("call")
        .arg("get:/pets")
        .output()
        .expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(json["ok"], true);
    assert_eq!(json["kind"], "call_result");
    assert_eq!(json["operation"], "get:/pets");
}

#[test]
fn list_outputs_operation_id_and_display_name() {
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
        .arg("--no-cache")
        .arg("list")
        .output()
        .expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let first = &json["data"]["operations"][0];
    assert_eq!(first["operation_id"], "get:/pets");
    assert_eq!(first["display_name"], "GET /pets");
}
