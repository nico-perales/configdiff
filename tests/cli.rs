//! End-to-end tests for the `configdiff` binary, covering exit codes and I/O.
//!
//! Cargo exposes the built binary's path via the `CARGO_BIN_EXE_<name>` env var,
//! so these run against the real executable with no extra dependencies.

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_configdiff"))
}

/// Writes `content` to a uniquely named temp file with the given extension and
/// returns its path.
fn temp_file(name: &str, content: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("configdiff_test_{}_{name}", std::process::id()));
    std::fs::write(&path, content).expect("write temp file");
    path
}

#[test]
fn exit_zero_when_equal() {
    let a = temp_file("eq_a.json", r#"{"a":1,"b":2}"#);
    let b = temp_file("eq_b.json", r#"{"b":2,"a":1}"#);
    let status = bin()
        .arg(&a)
        .arg(&b)
        .arg("--color")
        .arg("never")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
}

#[test]
fn exit_one_when_different() {
    let a = temp_file("df_a.json", r#"{"a":1}"#);
    let b = temp_file("df_b.json", r#"{"a":2}"#);
    let status = bin()
        .arg(&a)
        .arg(&b)
        .arg("--color")
        .arg("never")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn exit_two_on_parse_error() {
    let a = temp_file("er_a.json", "{ not valid");
    let b = temp_file("er_b.json", "{}");
    let out = bin()
        .arg(&a)
        .arg(&b)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("configdiff:"));
}

#[test]
fn exit_zero_flag_forces_success_despite_changes() {
    let a = temp_file("ez_a.json", r#"{"a":1}"#);
    let b = temp_file("ez_b.json", r#"{"a":2}"#);
    let status = bin()
        .arg(&a)
        .arg(&b)
        .arg("--exit-zero")
        .arg("--quiet")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
}

#[test]
fn quiet_suppresses_output() {
    let a = temp_file("q_a.json", r#"{"a":1}"#);
    let b = temp_file("q_b.json", r#"{"a":2}"#);
    let out = bin().arg(&a).arg(&b).arg("--quiet").output().unwrap();
    assert!(out.stdout.is_empty());
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn reads_new_document_from_stdin() {
    let a = temp_file("si_a.toml", "port = 8080\n");
    let mut child = bin()
        .arg(&a)
        .arg("-")
        .arg("--format")
        .arg("toml")
        .arg("--color")
        .arg("never")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"port = 9090\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("port"), "output was: {stdout}");
}

#[test]
fn json_output_is_machine_readable() {
    let a = temp_file("jo_a.json", r#"{"a":1}"#);
    let b = temp_file("jo_b.json", r#"{"a":2}"#);
    let out = bin()
        .arg(&a)
        .arg(&b)
        .arg("-o")
        .arg("json")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json output");
    assert_eq!(parsed["summary"]["total"], 1);
}

#[test]
fn fail_on_removed_ignores_a_pure_addition() {
    let a = temp_file("fo1_a.json", r#"{"a":1}"#);
    let b = temp_file("fo1_b.json", r#"{"a":1,"c":3}"#);
    // A key was added, but we only fail on removals: exit 0.
    let out = bin()
        .arg(&a)
        .arg(&b)
        .arg("--fail-on")
        .arg("removed")
        .arg("--color")
        .arg("never")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    // The change is still printed even though it does not trigger failure.
    assert!(String::from_utf8_lossy(&out.stdout).contains('c'));
}

#[test]
fn fail_on_matches_a_relevant_change() {
    let a = temp_file("fo2_a.json", r#"{"a":1,"b":2}"#);
    let b = temp_file("fo2_b.json", r#"{"a":1}"#);
    // `b` was removed and we fail on removals: exit 1.
    let status = bin()
        .arg(&a)
        .arg(&b)
        .arg("--fail-on")
        .arg("removed")
        .arg("--quiet")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn ini_and_env_formats_are_supported() {
    let a = temp_file("ie_a.ini", "[s]\nk = old\n");
    let b = temp_file("ie_b.ini", "[s]\nk = new\n");
    let status = bin()
        .arg(&a)
        .arg(&b)
        .arg("--color")
        .arg("never")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}
