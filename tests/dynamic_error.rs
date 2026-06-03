//! Failure surface for custom dynamic grammars.
//!
//! A missing or incompatible shared library must yield an actionable
//! [`contasty::AppError::CustomLang`], never a panic. Kept in its own test
//! binary because `ast-grep`'s `DynamicLang` registry is process-global and
//! set-once; an isolated process keeps that global pristine.

use std::fs;

use contasty::AppError;
use contasty::config::Config;

#[test]
fn missing_library_is_actionable_not_a_panic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("contasty.toml"),
        "[languages.jsonc]\n\
         libraryPath = \"does-not-exist.so\"\n\
         languageSymbol = \"tree_sitter_json\"\n\
         extensions = [\"jsonc\"]\n\
         rules = \"jsonc.yml\"\n",
    )
    .expect("write config");

    let config = Config::load(Some(&tmp.path().join("contasty.toml")), tmp.path());
    let files = contasty::resolve(&[tmp.path().to_path_buf()], tmp.path()).expect("resolve");
    let err = contasty::collect(&files, contasty::CategorySelection::default(), &config)
        .err()
        .expect("missing grammar must error");

    assert!(
        matches!(err, AppError::CustomLang(_)),
        "expected CustomLang error, got: {err:?}"
    );
}
