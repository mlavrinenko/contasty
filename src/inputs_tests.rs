use std::fs;

use super::*;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(&path, body).expect("write");
    path
}

fn with_mode(path: PathBuf, mode: IgnoreMode) -> (PathBuf, IgnoreMode) {
    (path, mode)
}

fn enable(path: PathBuf) -> (PathBuf, IgnoreMode) {
    with_mode(path, IgnoreMode::Enable)
}

#[test]
fn resolve_unions_and_dedups_file_and_folder() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let one = write(tmp.path(), "a.rs", "fn a() {}\n");
    write(tmp.path(), "b.rs", "fn b() {}\n");
    let args = [enable(tmp.path().to_path_buf()), enable(one)];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(files.len(), 2, "{files:?}");
}

#[test]
fn resolve_missing_path_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("nope.rs");
    let args = [enable(missing)];
    let err = resolve(&args, tmp.path()).expect_err("missing must error");
    assert!(matches!(err, AppError::Input(_)), "{err:?}");
}

#[test]
fn resolve_glob_matches_files_only() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "a.rs", "fn a() {}\n");
    write(tmp.path(), "b.rs", "fn b() {}\n");
    write(tmp.path(), "c.txt", "text\n");
    let args = [enable(tmp.path().join("*.rs"))];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(
        files
            .iter()
            .all(|path| path.extension().is_some_and(|ext| ext == "rs"))
    );
}

#[test]
fn resolve_glob_walks_matched_directory_subtree() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "crates/x/src/lib.rs", "fn x() {}\n");
    write(tmp.path(), "crates/y/src/lib.rs", "fn y() {}\n");
    write(tmp.path(), "crates/x/readme.md", "doc\n");
    let args = [enable(tmp.path().join("crates/*/src"))];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(files.iter().all(|path| path.ends_with("src/lib.rs")));
}

#[test]
fn resolve_unfolds_query_file_in_folder() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    write(tmp.path(), "lib/b.rs", "fn b() {}\n");
    write(tmp.path(), "api.cty.yaml", "rules: |\n  src\n");
    let args = [enable(tmp.path().to_path_buf())];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(
        files.len(),
        2,
        "walk adds lib/b.rs, query adds src/a.rs: {files:?}"
    );
    assert!(
        files
            .iter()
            .any(|path| path.to_str().is_some_and(|text| text.contains("src/a.rs")))
    );
}

#[test]
fn resolve_unfolds_query_file_as_direct_arg() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    write(tmp.path(), "lib/b.rs", "fn b() {}\n");
    let query = write(tmp.path(), "api.cty.yaml", "rules: |\n  src\n");
    let args = [enable(query)];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
}

#[test]
fn resolve_glob_zero_match_is_not_an_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "a.txt", "text\n");
    let args = [enable(tmp.path().join("*.rs"))];
    let files = resolve(&args, tmp.path()).expect("zero match warns");
    assert!(files.is_empty(), "{files:?}");
}

#[test]
fn normalize_resolves_dot_and_parent() {
    assert_eq!(
        normalize(Path::new("./src/../src/a.rs")),
        PathBuf::from("src/a.rs")
    );
    assert_eq!(normalize(Path::new("a/b/../c")), PathBuf::from("a/c"));
}

fn setup_gitignore_tree(tmp: &Path) -> (PathBuf, PathBuf) {
    // WalkBuilder respects .gitignore only inside a git repository.
    fs::create_dir_all(tmp.join(".git")).expect("mkdir .git");
    write(tmp, ".gitignore", "ignored.txt\n");
    let kept = write(tmp, "kept.rs", "fn kept() {}\n");
    let ignored = write(tmp, "ignored.txt", "fn ignored() {}\n");
    (kept, ignored)
}

#[test]
fn resolve_enable_respects_gitignore() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (_kept, _ignored) = setup_gitignore_tree(tmp.path());
    let args = [enable(tmp.path().to_path_buf())];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(
        files
            .first()
            .expect("one")
            .to_str()
            .is_some_and(|text| text.ends_with("kept.rs"))
    );
}

#[test]
fn resolve_disable_includes_ignored() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (_kept, _ignored) = setup_gitignore_tree(tmp.path());
    let args = [with_mode(tmp.path().to_path_buf(), IgnoreMode::Disable)];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert!(files.len() >= 2, "both files present: {files:?}");
}

#[test]
fn resolve_reverse_only_ignored() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (_kept, _ignored) = setup_gitignore_tree(tmp.path());
    let args = [with_mode(tmp.path().to_path_buf(), IgnoreMode::Reverse)];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(
        files
            .first()
            .expect("one")
            .to_str()
            .is_some_and(|text| text.ends_with("ignored.txt"))
    );
}

#[test]
fn resolve_mixed_modes_per_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (_kept, _ignored) = setup_gitignore_tree(tmp.path());
    let args = [
        enable(tmp.path().to_path_buf()),
        with_mode(tmp.path().to_path_buf(), IgnoreMode::Reverse),
    ];
    let files = resolve(&args, tmp.path()).expect("resolve");
    assert_eq!(
        files.len(),
        2,
        "enable gives kept, reverse gives ignored: {files:?}"
    );
}
