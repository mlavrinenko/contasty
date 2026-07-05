use std::fs;

use super::*;

fn mkdirs(project_dir: &Path) {
    fs::create_dir_all(project_dir.join(PROJECT_CONFIG_DIR)).expect("mkdir project/.contasty");
}

#[test]
fn project_default_path_is_dot_contasty_config_toml() {
    let tmp = tempfile::tempdir().expect("tempdir");
    mkdirs(tmp.path());
    fs::write(
        tmp.path().join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "[compact]\nelide_min_bytes = 42\n",
    )
    .expect("write");
    let config = Config::load(None, tmp.path(), None);
    assert_eq!(config.compact.elide_min_bytes, 42);
}

#[test]
fn cli_config_overrides_project_default_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    mkdirs(tmp.path());
    fs::write(
        tmp.path().join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "[compact]\nelide_min_bytes = 1\n",
    )
    .expect("write default location");
    let explicit = tmp.path().join("custom.toml");
    fs::write(&explicit, "[compact]\nelide_min_bytes = 2\n").expect("write explicit");
    let config = Config::load(Some(&explicit), tmp.path(), None);
    assert_eq!(
        config.compact.elide_min_bytes, 2,
        "--config wins over the default .contasty/config.toml"
    );
}

#[test]
fn compact_replaces_wholesale_when_project_sets_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let global_dir = tmp.path().join("global");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&global_dir).expect("mkdir global");
    mkdirs(&project_dir);
    fs::write(
        global_dir.join(CONFIG_FILE_NAME),
        "[compact]\nelide_min_bytes = 999\nmax_string_bytes = 999\n",
    )
    .expect("write global");
    fs::write(
        project_dir.join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "[compact]\nelide_min_bytes = 10\n",
    )
    .expect("write project");
    let config = Config::load(None, &project_dir, Some(&global_dir));
    assert_eq!(
        config.compact.elide_min_bytes, 10,
        "project's compact wins wholesale"
    );
    assert_eq!(
        config.compact.max_string_bytes, 256,
        "project's [compact] replaces global's wholesale, not merged field-by-field"
    );
}

#[test]
fn strip_falls_back_to_global_when_project_silent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let global_dir = tmp.path().join("global");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&global_dir).expect("mkdir global");
    mkdirs(&project_dir);
    fs::write(global_dir.join(CONFIG_FILE_NAME), "strip = [\"tests\"]\n").expect("write global");
    fs::write(
        project_dir.join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "[compact]\nelide_min_bytes = 5\n",
    )
    .expect("write project");
    let config = Config::load(None, &project_dir, Some(&global_dir));
    assert!(
        config.strip.expect("inherited from global").0.drop_tests(),
        "project set no [strip], global's carries through"
    );
}

#[test]
fn strip_project_wins_over_global_when_both_set() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let global_dir = tmp.path().join("global");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&global_dir).expect("mkdir global");
    mkdirs(&project_dir);
    fs::write(global_dir.join(CONFIG_FILE_NAME), "strip = [\"tests\"]\n").expect("write global");
    fs::write(
        project_dir.join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "strip = [\"comments\"]\n",
    )
    .expect("write project");
    let config = Config::load(None, &project_dir, Some(&global_dir));
    let strip = config.strip.expect("strip present").0;
    assert!(strip.drop_comments(), "project's strip wins wholesale");
    assert!(!strip.drop_tests(), "global's strip does not leak through");
}

#[test]
fn languages_union_by_key_project_wins_shared_key() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let global_dir = tmp.path().join("global");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&global_dir).expect("mkdir global");
    mkdirs(&project_dir);
    fs::write(
        global_dir.join(CONFIG_FILE_NAME),
        "[languages.rust]\nstrip = [\"tests\"]\n[languages.php]\nstrip = [\"comments\"]\n",
    )
    .expect("write global");
    fs::write(
        project_dir.join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "[languages.rust]\nstrip = [\"body\"]\n",
    )
    .expect("write project");
    let config = Config::load(None, &project_dir, Some(&global_dir));
    let rust = config.languages.get("rust").expect("rust from project");
    let rust_set = rust.strip.expect("rust strip").0;
    assert!(
        rust_set.drop_bodies(),
        "project's rust entry wins wholesale"
    );
    assert!(
        !rust_set.drop_tests(),
        "project's rust entry replaces global's, not merged"
    );
    let php = config
        .languages
        .get("php")
        .expect("php inherited from global");
    assert!(php.strip.expect("php strip").0.drop_comments());
}

#[test]
fn global_only_language_applies_with_no_project_config() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let global_dir = tmp.path().join("global");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&global_dir).expect("mkdir global");
    fs::create_dir_all(&project_dir).expect("mkdir project"); // no .contasty at all
    fs::write(
        global_dir.join(CONFIG_FILE_NAME),
        "[languages.rust]\nstrip = [\"tests\"]\n",
    )
    .expect("write global");
    let config = Config::load(None, &project_dir, Some(&global_dir));
    let rust = config
        .languages
        .get("rust")
        .expect("global-only language applies to a project with no config of its own");
    assert!(rust.strip.expect("strip").0.drop_tests());
}

#[test]
fn lang_paths_resolve_absolute_against_each_layers_own_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let global_dir = tmp.path().join("global");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&global_dir).expect("mkdir global");
    mkdirs(&project_dir);
    fs::write(
        global_dir.join(CONFIG_FILE_NAME),
        "[languages.rust]\nextend = \"rules/rust-extra.yml\"\n",
    )
    .expect("write global");
    fs::write(
        project_dir.join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "[languages.php]\noverride = \"rules/php-custom.yml\"\n",
    )
    .expect("write project");
    let config = Config::load(None, &project_dir, Some(&global_dir));
    let rust = config.languages.get("rust").expect("rust from global");
    assert_eq!(rust.extend, Some(global_dir.join("rules/rust-extra.yml")));
    let php = config.languages.get("php").expect("php from project");
    assert_eq!(
        php.r#override,
        Some(
            project_dir
                .join(PROJECT_CONFIG_DIR)
                .join("rules/php-custom.yml")
        )
    );
}

#[test]
fn library_path_single_and_platform_map_resolve_absolute() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("proj");
    mkdirs(&dir);
    fs::write(
        dir.join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        "[languages.mylang]\n\
         libraryPath = \"grammars/mylang.so\"\n\
         extensions = [\"ml\"]\n\
         rules = \"rules/mylang.yml\"\n\
         [languages.other]\n\
         extensions = [\"ot\"]\n\
         rules = \"rules/other.yml\"\n\
         [languages.other.libraryPath]\n\
         \"x86_64-unknown-linux-gnu\" = \"grammars/other-linux.so\"\n",
    )
    .expect("write");
    let config = Config::load(None, &dir, None);
    let base = dir.join(PROJECT_CONFIG_DIR);

    let mylang = config.languages.get("mylang").expect("mylang");
    match mylang.library_path.as_ref().expect("lib path") {
        LibraryPath::Single(path) => assert_eq!(*path, base.join("grammars/mylang.so")),
        LibraryPath::Platform(_) => panic!("expected a single library path"),
    }

    let other = config.languages.get("other").expect("other");
    match other.library_path.as_ref().expect("lib path") {
        LibraryPath::Platform(map) => assert_eq!(
            map.get("x86_64-unknown-linux-gnu").expect("target entry"),
            &base.join("grammars/other-linux.so")
        ),
        LibraryPath::Single(_) => panic!("expected a per-platform map"),
    }
}

#[test]
fn absolute_lang_path_is_left_unchanged() {
    let tmp = tempfile::tempdir().expect("tempdir");
    mkdirs(tmp.path());
    let absolute = tmp.path().join("elsewhere/rust-extra.yml");
    fs::write(
        tmp.path().join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
        format!(
            "[languages.rust]\nextend = {:?}\n",
            absolute.to_str().expect("utf8 tempdir path")
        ),
    )
    .expect("write");
    let config = Config::load(None, tmp.path(), None);
    let rust = config.languages.get("rust").expect("rust");
    assert_eq!(rust.extend, Some(absolute));
}
