//! A dynamic grammar defined only in the *global* config must still apply to
//! a project that sets no grammar of its own — proves per-language paths
//! resolve absolute against each layer's own defining config file's
//! directory, not a single shared `Config.base` (which no longer exists).
//!
//! Gated and isolated exactly like `tests/dynamic.rs`: ast-grep ships
//! `json-linux.so` for linux `x86_64` only, and `DynamicLang` registration is
//! process-global and set-once, so this lives in its own test binary.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::fs;
use std::path::Path;

use contasty::config::Config;

const FIXTURE_SO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/json-linux.so");

/// Lay down a global contasty dir wiring up the `jsonc` grammar: the compiled
/// grammar, `config.toml`, and its rule file. No project-level config exists
/// anywhere in this test.
fn scaffold_global(global_dir: &Path) {
    fs::create_dir_all(global_dir).expect("mkdir global");
    fs::copy(FIXTURE_SO, global_dir.join("json-linux.so")).expect("copy fixture grammar");
    fs::write(
        global_dir.join("config.toml"),
        "[languages.jsonc]\n\
         libraryPath = \"json-linux.so\"\n\
         languageSymbol = \"tree_sitter_json\"\n\
         extensions = [\"jsonc\"]\n\
         rules = \"jsonc.yml\"\n",
    )
    .expect("write global config");
    fs::write(
        global_dir.join("jsonc.yml"),
        "language: jsonc\n\
         rules:\n  \
           - action: truncate\n    \
             rule:\n      \
               kind: string\n",
    )
    .expect("write rule file");
}

#[test]
fn global_only_dynamic_grammar_applies_to_any_project() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let global_dir = tmp.path().join("global");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&project_dir).expect("mkdir project");
    scaffold_global(&global_dir);
    fs::write(
        project_dir.join("sample.jsonc"),
        "{\"greeting\": \"hello world, this is a long value\"}\n",
    )
    .expect("write sample");

    // No project-level config anywhere — the grammar comes purely from global.
    let config = Config::load(None, &project_dir, Some(&global_dir));
    let files = contasty::resolve(
        &[(project_dir.clone(), contasty::IgnoreMode::Enable, None)],
        &project_dir,
        Some(&global_dir),
    )
    .expect("resolve");
    let items = contasty::collect(&files, &config).expect("collect");

    let jsonc = items
        .iter()
        .find(|item| item.path.extension().is_some_and(|ext| ext == "jsonc"))
        .expect("jsonc file stripped");

    assert_eq!(jsonc.lang_name, "jsonc");
    assert!(jsonc.content.contains("[…CTY]"), "got: {}", jsonc.content);
    assert!(
        !jsonc.content.contains("hello world"),
        "value not stripped: {}",
        jsonc.content
    );
}
