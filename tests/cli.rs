//! End-to-end CLI test: invoke the binary on a temp dir and inspect stdout.

use std::fs;

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn cli_strips_rust_function_body_and_renders_markdown() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("sample.rs"),
        "pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n",
    )
    .expect("write");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("sample.rs"))
        .stdout(contains("```rust"))
        .stdout(contains("pub fn add(lhs: i32, rhs: i32) -> i32"))
        .stdout(contains("/* ... */"));
}

#[test]
fn cli_ignores_non_source_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("notes.txt"), "irrelevant\n").expect("write");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}
