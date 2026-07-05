use std::path::Path;

use crate::config::{CompactConfig, Config};

use super::*;

const STRUCT_RULE: &str =
    "language: rust\nrules:\n  - action: delete\n    rule:\n      kind: struct_item\n";
const SRC_STRUCT_FN: &str = "struct Foo { x: i32 }\nfn add(a: i32, b: i32) -> i32 { a + b }\n";

/// A `Config` whose only content is one `[languages.<lang>]` entry. Paths in
/// `entry` must already be absolute — production code resolves them at
/// `Config::load` time, so a hand-built `Config` (bypassing that step) must
/// do the same itself.
fn config_with_rule(lang: &str, entry: LangConfig) -> Config {
    let mut languages = HashMap::new();
    languages.insert(lang.to_owned(), entry);
    Config {
        languages,
        ..Config::default()
    }
}

fn strip_rust(reg: &Registry, src: &str) -> String {
    reg.detect(Path::new("x.rs"))
        .expect("rust")
        .strip(
            src,
            Path::new("x.rs"),
            false,
            false,
            false,
            true,
            &CompactConfig::default(),
        )
        .expect("strip")
}

#[test]
fn extend_adds_a_rule_and_keeps_builtins() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("extra.yml"), STRUCT_RULE).expect("write");
    let config = config_with_rule(
        "rust",
        LangConfig {
            extend: Some(dir.path().join("extra.yml")),
            ..LangConfig::default()
        },
    );
    let reg = Registry::with_config(&config).expect("registry");
    let out = strip_rust(&reg, SRC_STRUCT_FN);
    assert!(
        !out.contains("struct Foo"),
        "extend rule did not fire: {out}"
    );
    assert!(
        out.contains("fn add(a: i32, b: i32) -> i32"),
        "builtin signature gone: {out}"
    );
    assert!(out.contains("{}"), "builtin body-elide gone: {out}");
    assert!(!out.contains("a + b"), "builtin kept the body: {out}");
}

#[test]
fn override_replaces_the_whole_set() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("only.yml"), STRUCT_RULE).expect("write");
    let config = config_with_rule(
        "rust",
        LangConfig {
            r#override: Some(dir.path().join("only.yml")),
            ..LangConfig::default()
        },
    );
    let reg = Registry::with_config(&config).expect("registry");
    let out = strip_rust(&reg, SRC_STRUCT_FN);
    assert!(
        !out.contains("struct Foo"),
        "override rule did not fire: {out}"
    );
    // The built-in fn-body elide is gone under override, so the body survives.
    assert!(
        out.contains("a + b"),
        "builtin body-elide still active: {out}"
    );
}

#[test]
fn both_extend_and_override_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = config_with_rule(
        "rust",
        LangConfig {
            extend: Some(dir.path().join("a.yml")),
            r#override: Some(dir.path().join("b.yml")),
            ..LangConfig::default()
        },
    );
    let Err(err) = Registry::with_config(&config) else {
        panic!("both keys must be rejected");
    };
    assert!(
        err.to_string().contains("choose one"),
        "unexpected error: {err}"
    );
}

#[test]
fn extend_file_language_must_match_table_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("wrong.yml"),
        "language: php\nrules:\n  - action: delete\n    rule:\n      kind: comment\n",
    )
    .expect("write");
    let config = config_with_rule(
        "rust",
        LangConfig {
            extend: Some(dir.path().join("wrong.yml")),
            ..LangConfig::default()
        },
    );
    let Err(err) = Registry::with_config(&config) else {
        panic!("language mismatch must error");
    };
    assert!(
        err.to_string().contains("expected `rust`"),
        "unexpected error: {err}"
    );
}

#[test]
fn unknown_language_in_rules_table_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = config_with_rule(
        "nonexistent-lang",
        LangConfig {
            extend: Some(dir.path().join("x.yml")),
            ..LangConfig::default()
        },
    );
    let Err(err) = Registry::with_config(&config) else {
        panic!("unknown language must error");
    };
    assert!(
        err.to_string().contains("unknown language"),
        "unexpected error: {err}"
    );
}
