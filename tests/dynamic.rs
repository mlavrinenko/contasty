//! Custom dynamic-grammar stripping, end to end through the public API.
//!
//! Registers a user-supplied tree-sitter grammar (`ast-grep`'s json fixture,
//! exposed under the `jsonc` extension via a symbol override) from a
//! `contasty.toml`, then strips a `.jsonc` file with a rule file — no rebuild.
//!
//! Gated to the one target the committed fixture is built for. ast-grep ships
//! `json-linux.so` for linux `x86_64`; other targets compile this file to nothing
//! and skip cleanly, mirroring ast-grep's own dynamic-language test.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::fs;
use std::path::Path;

use contasty::config::Config;

const FIXTURE_SO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/json-linux.so");

/// Lay down a self-contained project: the grammar, a `contasty.toml` wiring it
/// up, a rule file, and one sample source file. Returns the config base dir.
fn scaffold(dir: &Path) {
    fs::copy(FIXTURE_SO, dir.join("json-linux.so")).expect("copy fixture grammar");
    fs::write(
        dir.join("contasty.toml"),
        "[customLanguages.jsonc]\n\
         libraryPath = \"json-linux.so\"\n\
         languageSymbol = \"tree_sitter_json\"\n\
         extensions = [\"jsonc\"]\n\
         rules = \"jsonc.yml\"\n",
    )
    .expect("write config");
    fs::write(
        dir.join("jsonc.yml"),
        "language: jsonc\n\
         rules:\n  \
           - action: truncate\n    \
             rule:\n      \
               kind: string\n",
    )
    .expect("write rule file");
    fs::write(
        dir.join("sample.jsonc"),
        "{\"greeting\": \"hello world, this is a long value\"}\n",
    )
    .expect("write sample");
}

#[test]
fn custom_grammar_strips_via_rule_file_no_rebuild() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    let config = Config::load(Some(&tmp.path().join("contasty.toml")), tmp.path());
    let items = contasty::collect(tmp.path(), contasty::CategorySelection::default(), &config)
        .expect("collect");

    let jsonc = items
        .iter()
        .find(|item| item.path.extension().is_some_and(|ext| ext == "jsonc"))
        .expect("jsonc file stripped");

    assert_eq!(jsonc.lang_name, "jsonc");
    // tree-sitter-json `string` nodes (key and value) collapse to the marker.
    assert!(jsonc.content.contains("[…CTY]"), "got: {}", jsonc.content);
    assert!(
        !jsonc.content.contains("hello world"),
        "value not stripped: {}",
        jsonc.content
    );
}
