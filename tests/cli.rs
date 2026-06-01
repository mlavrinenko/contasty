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
fn cli_drops_comments_by_default_and_keeps_them_with_include() {
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
        .arg("--include=comments")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("/// doc for greet"))
        .stdout(contains("// trailing note"));
}

#[test]
fn cli_renders_json_with_format_flag() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("sample.rs"),
        "pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n",
    )
    .expect("write");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--format=json")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("\"lang\": \"rust\""))
        .stdout(contains("\"content\":"))
        .stdout(contains("pub fn add(lhs: i32, rhs: i32) -> i32"))
        .stdout(contains("```rust").not());
}

#[test]
fn cli_exclude_imports_drops_use() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("sample.rs"),
        "use std::collections::HashMap;\npub fn greet() {}\n",
    )
    .expect("write");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--exclude=imports")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("use std::collections::HashMap").not())
        .stdout(contains("pub fn greet"));
}

#[test]
fn cli_exclude_all_then_include_comments() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("sample.rs"),
        "/// doc\nuse std::fmt;\npub fn greet() {}\n",
    )
    .expect("write");

    // comments kept, imports excluded (all excluded first, then comments re-included)
    Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--exclude=all")
        .arg("--include=comments")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("/// doc"))
        .stdout(contains("use std::fmt").not());
}

#[test]
fn cli_include_everything_then_exclude_imports() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("sample.rs"),
        "/// doc\nuse std::fmt;\npub fn greet() {}\n",
    )
    .expect("write");

    // comments kept, imports excluded
    Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--include=everything")
        .arg("--exclude=imports")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("/// doc"))
        .stdout(contains("use std::fmt").not());
}

#[test]
fn cli_everything_alias_equals_all() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("sample.rs"),
        "/// doc\nuse std::fmt;\npub fn greet() {}\n",
    )
    .expect("write");

    let all_out = Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--include=all")
        .arg(tmp.path())
        .output()
        .expect("all");

    let everything_out = Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--include=everything")
        .arg(tmp.path())
        .output()
        .expect("everything");

    assert_eq!(all_out.stdout, everything_out.stdout);
}
