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

#[test]
fn cli_accepts_multiple_path_arguments() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("a.rs"), "pub fn a() {}\n").expect("write a");
    fs::write(tmp.path().join("b.rs"), "pub fn b() {}\n").expect("write b");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path().join("a.rs"))
        .arg(tmp.path().join("b.rs"))
        .assert()
        .success()
        .stdout(contains("a.rs"))
        .stdout(contains("b.rs"))
        .stdout(contains("pub fn a"))
        .stdout(contains("pub fn b"));
}

#[test]
fn cli_expands_glob_internally() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("a.rs"), "pub fn a() {}\n").expect("write a");
    fs::write(tmp.path().join("b.rs"), "pub fn b() {}\n").expect("write b");
    // a supported non-match: present in the dir but outside the `*.rs` glob.
    fs::write(tmp.path().join("other.py"), "def other():\n    pass\n").expect("write py");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path().join("*.rs"))
        .assert()
        .success()
        .stdout(contains("a.rs"))
        .stdout(contains("b.rs"))
        .stdout(contains("other.py").not());
}

#[test]
fn cli_missing_path_is_an_error() {
    let tmp = tempfile::tempdir().expect("tempdir");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path().join("does-not-exist.rs"))
        .assert()
        .failure();
}

#[test]
fn cli_query_file_unfolds_to_selected_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    fs::create_dir_all(tmp.path().join("lib")).expect("mkdir lib");
    fs::write(tmp.path().join("src/a.rs"), "pub fn a() {}\n").expect("write a");
    fs::write(tmp.path().join("lib/b.rs"), "pub fn b() {}\n").expect("write b");
    fs::write(tmp.path().join("api.cty.yaml"), "rules: |\n  src\n").expect("write query");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path().join("api.cty.yaml"))
        .assert()
        .success()
        .stdout(contains("a.rs"))
        .stdout(contains("pub fn a"))
        .stdout(contains("b.rs").not());
}

#[test]
fn cli_query_file_negation_excludes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    fs::write(tmp.path().join("src/keep.rs"), "pub fn keep() {}\n").expect("write keep");
    fs::write(tmp.path().join("src/drop.rs"), "pub fn drop() {}\n").expect("write drop");
    fs::write(
        tmp.path().join("q.cty.yaml"),
        "rules: |\n  src\n  !src/drop.rs\n",
    )
    .expect("write query");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path().join("q.cty.yaml"))
        .assert()
        .success()
        .stdout(contains("keep.rs"))
        .stdout(contains("drop.rs").not());
}

fn init_git_repo(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join(".git")).expect("mkdir .git");
}

#[test]
fn cli_default_respects_gitignore() {
    let tmp = tempfile::tempdir().expect("tempdir");
    init_git_repo(tmp.path());
    fs::write(tmp.path().join(".gitignore"), "ignored.rs\n").expect("write gitignore");
    fs::write(tmp.path().join("kept.rs"), "pub fn kept() {}\n").expect("write kept");
    fs::write(tmp.path().join("ignored.rs"), "pub fn ignored() {}\n").expect("write ignored");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("kept.rs"))
        .stdout(contains("ignored.rs").not());
}

#[test]
fn cli_ignore_disable_includes_ignored() {
    let tmp = tempfile::tempdir().expect("tempdir");
    init_git_repo(tmp.path());
    fs::write(tmp.path().join(".gitignore"), "ignored.rs\n").expect("write gitignore");
    fs::write(tmp.path().join("kept.rs"), "pub fn kept() {}\n").expect("write kept");
    fs::write(tmp.path().join("ignored.rs"), "pub fn ignored() {}\n").expect("write ignored");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--ignore=disable")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("kept.rs"))
        .stdout(contains("ignored.rs"));
}

#[test]
fn cli_ignore_reverse_only_ignored() {
    let tmp = tempfile::tempdir().expect("tempdir");
    init_git_repo(tmp.path());
    fs::write(tmp.path().join(".gitignore"), "ignored.rs\n").expect("write gitignore");
    fs::write(tmp.path().join("kept.rs"), "pub fn kept() {}\n").expect("write kept");
    fs::write(tmp.path().join("ignored.rs"), "pub fn ignored() {}\n").expect("write ignored");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg("--ignore=reverse")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(contains("ignored.rs"))
        .stdout(contains("kept.rs").not());
}

#[test]
fn cli_ignore_interleave_mode_switch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    init_git_repo(tmp.path());
    fs::write(tmp.path().join(".gitignore"), "ignored.rs\n").expect("write gitignore");
    fs::write(tmp.path().join("kept.rs"), "pub fn kept() {}\n").expect("write kept");
    fs::write(tmp.path().join("ignored.rs"), "pub fn ignored() {}\n").expect("write ignored");

    // First arg uses enable (default), second uses reverse.
    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path().join("kept.rs"))
        .arg("--ignore=reverse")
        .arg(tmp.path().join("ignored.rs"))
        .assert()
        .success()
        .stdout(contains("kept.rs"))
        .stdout(contains("ignored.rs"));
}

#[test]
fn cli_query_ignore_field_wins() {
    let tmp = tempfile::tempdir().expect("tempdir");
    init_git_repo(tmp.path());
    fs::write(tmp.path().join(".gitignore"), "generated/\n").expect("write gitignore");
    fs::create_dir_all(tmp.path().join("generated")).expect("mkdir generated");
    fs::write(tmp.path().join("generated/foo.rs"), "pub fn foo() {}\n").expect("write generated");
    // Query sets ignore: disable so it can reach the gitignored directory.
    fs::write(
        tmp.path().join("gen.cty.yaml"),
        "ignore: disable\nrules: |\n  generated\n",
    )
    .expect("write query");

    Command::cargo_bin("contasty")
        .expect("binary")
        .arg(tmp.path().join("gen.cty.yaml"))
        .assert()
        .success()
        .stdout(contains("foo.rs"));
}
