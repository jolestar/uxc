use assert_cmd::Command;

fn uxc_command() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("uxc"))
}

#[test]
fn short_v_prints_version() {
    let output = uxc_command()
        .arg("-v")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout should be UTF-8");
    assert_eq!(stdout.trim(), format!("uxc {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_token_prints_version() {
    let output = uxc_command()
        .arg("version")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout should be UTF-8");
    assert_eq!(stdout.trim(), format!("uxc {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn long_version_still_works() {
    let output = uxc_command()
        .arg("--version")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout should be UTF-8");
    assert_eq!(stdout.trim(), format!("uxc {}", env!("CARGO_PKG_VERSION")));
}
