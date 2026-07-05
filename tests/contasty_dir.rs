//! End-to-end coverage for the `.contasty/` project dir and the XDG global
//! config/queries layer: default project config path, project-over-global
//! layering, `@name` saved-query resolution (project then global), and a
//! saved query's patterns rooting at the scanned project rather than at the
//! query file's own directory.

use std::fs;

use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;
use predicates::str::contains;

/// `sample.rs` with one of each: a doc comment, an import, and a body.
const SAMPLE_RS: &str =
    "/// doc comment\nuse std::fmt;\npub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n";

#[test]
fn project_config_default_location_is_dot_contasty_config_toml() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("sample.rs"), SAMPLE_RS).expect("write sample");
    fs::create_dir_all(tmp.path().join(".contasty")).expect("mkdir .contasty");
    fs::write(
        tmp.path().join(".contasty/config.toml"),
        "strip = [\"comments\"]\n",
    )
    .expect("write project config");

    Command::cargo_bin("contasty")
        .expect("binary")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(contains("/// doc comment").not())
        .stdout(contains("use std::fmt"))
        .stdout(contains("lhs + rhs"));
}

#[test]
fn xdg_global_config_applies_when_project_has_none() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project_dir = tmp.path().join("project");
    let xdg_dir = tmp.path().join("xdg");
    fs::create_dir_all(&project_dir).expect("mkdir project");
    fs::write(project_dir.join("sample.rs"), SAMPLE_RS).expect("write sample");
    fs::create_dir_all(xdg_dir.join("contasty")).expect("mkdir xdg/contasty");
    fs::write(
        xdg_dir.join("contasty/config.toml"),
        "strip = [\"comments\"]\n",
    )
    .expect("write global config");

    Command::cargo_bin("contasty")
        .expect("binary")
        .current_dir(&project_dir)
        .env("XDG_CONFIG_HOME", &xdg_dir)
        .assert()
        .success()
        .stdout(contains("/// doc comment").not())
        .stdout(contains("use std::fmt"))
        .stdout(contains("lhs + rhs"));
}

#[test]
fn project_config_wins_over_global_on_a_shared_key() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project_dir = tmp.path().join("project");
    let xdg_dir = tmp.path().join("xdg");
    fs::create_dir_all(&project_dir).expect("mkdir project");
    fs::write(project_dir.join("sample.rs"), SAMPLE_RS).expect("write sample");
    fs::create_dir_all(xdg_dir.join("contasty")).expect("mkdir xdg/contasty");
    // Global says strip nothing; project overrides with strip=[comments].
    fs::write(xdg_dir.join("contasty/config.toml"), "strip = []\n").expect("write global config");
    fs::create_dir_all(project_dir.join(".contasty")).expect("mkdir .contasty");
    fs::write(
        project_dir.join(".contasty/config.toml"),
        "strip = [\"comments\"]\n",
    )
    .expect("write project config");

    Command::cargo_bin("contasty")
        .expect("binary")
        .current_dir(&project_dir)
        .env("XDG_CONFIG_HOME", &xdg_dir)
        .assert()
        .success()
        .stdout(contains("/// doc comment").not())
        .stdout(contains("use std::fmt"))
        .stdout(contains("lhs + rhs"));
}

#[test]
fn named_query_resolves_from_project_queries_dir_and_roots_at_project() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    fs::create_dir_all(tmp.path().join("lib")).expect("mkdir lib");
    fs::write(tmp.path().join("src/a.rs"), "pub fn a() {}\n").expect("write a");
    fs::write(tmp.path().join("lib/b.rs"), "pub fn b() {}\n").expect("write b");
    // The saved query lives under .contasty/queries/, not the project root,
    // but its `src` pattern must still select the project's own src/.
    fs::create_dir_all(tmp.path().join(".contasty/queries")).expect("mkdir queries");
    fs::write(
        tmp.path().join(".contasty/queries/api.cty.yaml"),
        "rules: |\n  src\n",
    )
    .expect("write query");

    Command::cargo_bin("contasty")
        .expect("binary")
        .current_dir(tmp.path())
        .arg("@api")
        .assert()
        .success()
        .stdout(contains("a.rs"))
        .stdout(contains("pub fn a"))
        .stdout(contains("b.rs").not());
}

#[test]
fn named_query_falls_back_to_global_queries_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project_dir = tmp.path().join("project");
    let xdg_dir = tmp.path().join("xdg");
    fs::create_dir_all(project_dir.join("src")).expect("mkdir src");
    fs::write(project_dir.join("src/a.rs"), "pub fn a() {}\n").expect("write a");
    fs::create_dir_all(xdg_dir.join("contasty/queries")).expect("mkdir global queries");
    fs::write(
        xdg_dir.join("contasty/queries/api.cty.yaml"),
        "rules: |\n  src\n",
    )
    .expect("write global query");

    Command::cargo_bin("contasty")
        .expect("binary")
        .current_dir(&project_dir)
        .env("XDG_CONFIG_HOME", &xdg_dir)
        .arg("@api")
        .assert()
        .success()
        .stdout(contains("a.rs"))
        .stdout(contains("pub fn a"));
}

#[test]
fn named_query_not_found_is_an_error() {
    let tmp = tempfile::tempdir().expect("tempdir");

    Command::cargo_bin("contasty")
        .expect("binary")
        .current_dir(tmp.path())
        .arg("@does-not-exist")
        .assert()
        .failure()
        .stderr(contains("@does-not-exist"));
}
