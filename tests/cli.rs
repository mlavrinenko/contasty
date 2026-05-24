//! End-to-end CLI test: invoke the binary on a temp dir and inspect stdout.

use std::fs;

use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;
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
        .stdout(contains("{}"));
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

#[test]
fn cli_drops_comments_by_default_and_keeps_them_with_include_comments() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("sample.rs"),
        "/// doc for greet\npub fn greet() {}\n// trailing note\n",
    )
    .expect("write");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("pub fn greet"))
        .stdout(contains("doc for greet").not())
        .stdout(contains("trailing note").not());

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--include-comments")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("/// doc for greet"))
        .stdout(contains("// trailing note"));
}
