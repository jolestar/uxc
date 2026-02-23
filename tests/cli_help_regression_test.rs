use std::process::Command;

fn uxc_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_uxc"))
}

#[test]
fn call_subcommand_help_does_not_panic() {
    let output = uxc_command()
        .arg("call")
        .arg("--help")
        .output()
        .expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Execute an operation"));
}

#[test]
fn operation_help_flag_no_longer_conflicts_with_clap_help() {
    let output = uxc_command()
        .arg("https://example.com")
        .arg("call")
        .arg("ping")
        .arg("--help")
        .output()
        .expect("failed to run uxc");

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--op-help"));
}
