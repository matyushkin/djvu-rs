//! CLI coverage for `djvu validate` human output, strict exits, and JSON schema.

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn warning_only_input() -> NamedTempFile {
    // This real legacy fixture omits final alignment padding after its
    // odd-length BG44 stream. The validator recovers it as a warning without
    // treating the document as invalid.
    let data = std::fs::read(fixture("boy.djvu")).expect("fixture exists");
    let file = NamedTempFile::new().expect("temporary file");
    std::fs::write(file.path(), data).expect("write temporary DjVu");
    file
}

#[test]
fn strict_turns_a_warning_only_file_into_exit_one() {
    let file = warning_only_input();
    Command::cargo_bin("djvu")
        .expect("binary builds")
        .args(["validate", file.path().to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicates::str::contains("1 warnings"));
    Command::cargo_bin("djvu")
        .expect("binary builds")
        .args([
            "validate",
            file.path().to_str().expect("utf8 path"),
            "--strict",
        ])
        .assert()
        .code(1);
}

#[test]
fn json_has_stable_schema_and_consistent_summary() {
    let output = Command::cargo_bin("djvu")
        .expect("binary builds")
        .args([
            "validate",
            fixture("boy.djvu").to_str().expect("utf8 fixture path"),
            "--json",
        ])
        .output()
        .expect("run validate");
    assert!(output.status.success(), "{output:?}");
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert!(json["file"].is_string());
    assert!(json["valid"].is_boolean());
    assert!(json["summary"].is_object());
    assert!(json["findings"].is_array());
    let findings = json["findings"].as_array().expect("findings array");
    for finding in findings {
        for field in [
            "severity",
            "layer",
            "code",
            "component",
            "chunk",
            "offset",
            "message",
        ] {
            assert!(finding.get(field).is_some(), "missing {field}: {finding}");
        }
    }
    let summary = json["summary"].as_object().expect("summary object");
    let total: usize = ["errors", "warnings", "tolerated", "recovery"]
        .into_iter()
        .map(|field| summary[field].as_u64().expect("count") as usize)
        .sum();
    assert_eq!(total, findings.len());
}

#[test]
fn unreadable_file_exits_two() {
    Command::cargo_bin("djvu")
        .expect("binary builds")
        .args(["validate", "/tmp/djvu-rs-validate-does-not-exist.djvu"])
        .assert()
        .code(2);
}
