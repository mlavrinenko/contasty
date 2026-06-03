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

#[test]
fn query_inline_rules_unfolds_matching_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    write(tmp.path(), "src/b.rs", "fn b() {}\n");
    write(tmp.path(), "lib/c.rs", "fn c() {}\n");
    let query = write(tmp.path(), "api.cty.yaml", "rules: |\n  src\n");
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(
        files
            .iter()
            .all(|path| path.to_str().is_some_and(|text| text.contains("src")))
    );
}

#[test]
fn query_negation_excludes_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/keep.rs", "fn k() {}\n");
    write(tmp.path(), "src/drop.rs", "fn d() {}\n");
    let body = "rules: |\n  src\n  !src/drop.rs\n";
    let query = write(tmp.path(), "q.cty.yaml", body);
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(
        files
            .first()
            .expect("one")
            .to_str()
            .is_some_and(|t| t.contains("keep.rs"))
    );
}

#[test]
fn query_list_form_rules() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    write(tmp.path(), "src/b_test.rs", "fn b() {}\n");
    let body = "rules:\n  - \"src/**/*.rs\"\n  - \"!**/*_test.rs\"\n";
    let query = write(tmp.path(), "q.cty.yaml", body);
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(
        files
            .first()
            .expect("one")
            .to_str()
            .is_some_and(|t| t.ends_with("a.rs"))
    );
}

#[test]
fn query_external_rules_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    write(tmp.path(), "src/b.rs", "fn b() {}\n");
    write(tmp.path(), "special.ignore", "src/a.rs\n");
    let body = "rules:\n  path: ./special.ignore\n";
    let query = write(tmp.path(), "q.cty.yaml", body);
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(
        files
            .first()
            .expect("one")
            .to_str()
            .is_some_and(|t| t.ends_with("a.rs"))
    );
}

#[test]
fn query_import_unions_results() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    write(tmp.path(), "lib/b.rs", "fn b() {}\n");
    write(tmp.path(), "shared.cty.yaml", "rules: |\n  lib\n");
    let body = "rules: |\n  src\nimport:\n  - shared.cty.yaml\n";
    let query = write(tmp.path(), "main.cty.yaml", body);
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 2, "{files:?}");
}

#[test]
fn query_missing_required_import_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let body = "import:\n  - missing.cty.yaml\n";
    let query = write(tmp.path(), "q.cty.yaml", body);
    let mut visited = BTreeSet::new();
    let err = resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited)
        .expect_err("missing required import must error");
    assert!(matches!(err, AppError::Query(_)), "{err:?}");
}

#[test]
fn query_optional_import_skips_silently() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    let body = "rules: |\n  src\nimport:\n  - path: missing.cty.yaml\n    required: false\n";
    let query = write(tmp.path(), "q.cty.yaml", body);
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
}

#[test]
fn query_cycle_guard_prevents_infinite_recursion() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/a.rs", "fn a() {}\n");
    write(
        tmp.path(),
        "a.cty.yaml",
        "rules: |\n  src\nimport:\n  - b.cty.yaml\n",
    );
    write(tmp.path(), "b.cty.yaml", "import:\n  - a.cty.yaml\n");
    let query = tmp.path().join("a.cty.yaml");
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
}

#[test]
fn query_rules_matched_query_file_unfolds() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "real.rs", "fn r() {}\n");
    write(tmp.path(), "sub.cty.yaml", "rules: |\n  real.rs\n");
    // main selects only the sub-query file; its files arrive via unfold,
    // not by being emitted as YAML content.
    let query = write(tmp.path(), "main.cty.yaml", "rules: |\n  sub.cty.yaml\n");
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(
        files
            .first()
            .expect("one")
            .to_str()
            .is_some_and(|t| t.ends_with("real.rs"))
    );
}

#[test]
fn query_rules_intersect_mode_gates_ignored_files() {
    // Mode picks the candidate set; rules select within it. Under `enable`
    // a rule cannot re-include a `.gitignore`d file; under `disable` it can.
    let tmp = tempfile::tempdir().expect("tempdir");
    // WalkBuilder respects .gitignore only inside a git repository.
    fs::create_dir_all(tmp.path().join(".git")).expect("mkdir .git");
    write(tmp.path(), ".gitignore", "generated/\n");
    write(tmp.path(), "generated/foo.rs", "fn foo() {}\n");
    let query = write(tmp.path(), "q.cty.yaml", "rules: |\n  generated\n");

    let mut visited = BTreeSet::new();
    let enabled =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert!(
        enabled.is_empty(),
        "enable mode must not reach gitignored files: {enabled:?}"
    );

    let mut visited = BTreeSet::new();
    let disabled =
        resolve_query(&query, IgnoreMode::Disable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(
        disabled.len(),
        1,
        "disable mode admits ignored: {disabled:?}"
    );
}

#[test]
fn query_rules_cycle_guard_holds() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "real.rs", "fn r() {}\n");
    write(
        tmp.path(),
        "a.cty.yaml",
        "rules: |\n  real.rs\n  b.cty.yaml\n",
    );
    write(tmp.path(), "b.cty.yaml", "rules: |\n  a.cty.yaml\n");
    let query = tmp.path().join("a.cty.yaml");
    let mut visited = BTreeSet::new();
    let files =
        resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited).expect("resolve");
    assert_eq!(files.len(), 1, "{files:?}");
}

#[test]
fn query_path_escape_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let body = "import:\n  - ../../etc/something.cty.yaml\n";
    let query = write(tmp.path(), "q.cty.yaml", body);
    let mut visited = BTreeSet::new();
    let err = resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited)
        .expect_err("escape must error");
    assert!(matches!(err, AppError::Query(_)), "{err:?}");
}

#[test]
fn query_broken_yaml_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let query = write(tmp.path(), "q.cty.yaml", "not: [valid: yaml");
    let mut visited = BTreeSet::new();
    let err = resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited)
        .expect_err("broken yaml must error");
    assert!(matches!(err, AppError::Query(_)), "{err:?}");
}

#[test]
fn query_unknown_field_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let query = write(tmp.path(), "q.cty.yaml", "rules: |\n  src\nbogus: true\n");
    let mut visited = BTreeSet::new();
    let err = resolve_query(&query, IgnoreMode::Enable, tmp.path(), &mut visited)
        .expect_err("unknown field must error");
    assert!(matches!(err, AppError::Query(_)), "{err:?}");
}
