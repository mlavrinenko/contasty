use std::io::Write;
use std::path::Path;

use super::*;

#[test]
fn config_defaults_when_file_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = Config::load(None, dir.path(), None);
    assert_eq!(config.compact.elide_min_bytes, 0);
}

#[test]
fn config_loads_from_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    let mut f = std::fs::File::create(&path).expect("create");
    writeln!(f, "[compact]\nelide_min_bytes = 256").expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    assert_eq!(config.compact.elide_min_bytes, 256);
}

#[test]
fn config_defaults_when_invalid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    std::fs::write(&path, "not valid toml {{{").expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    assert_eq!(config.compact.elide_min_bytes, default_min_bytes());
}

#[test]
fn builtin_defaults_apply_when_no_config_or_cli() {
    let config = Config::default();
    let strip = config.resolve_config_strip("rust");
    assert!(strip.drop_comments(), "comments stripped by default");
    assert!(strip.drop_imports(), "imports stripped by default");
    assert!(!strip.drop_tests(), "tests kept by default");
    assert!(strip.drop_bodies(), "bodies stripped by default");
}

#[test]
fn strip_cross_language_defaults_apply_to_all_langs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    std::fs::write(&path, "strip = [\"tests\"]\n").expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    assert!(config.resolve_config_strip("rust").drop_tests());
    assert!(config.resolve_config_strip("php").drop_tests());
}

#[test]
fn strip_per_language_overrides_cross_language() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    std::fs::write(
        &path,
        "strip = [\"comments\"]\n\
         [languages.rust]\n\
         strip = [\"tests\"]\n",
    )
    .expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    let rust = config.resolve_config_strip("rust");
    assert!(!rust.drop_comments(), "rust: comments kept (per-lang wins)");
    assert!(rust.drop_tests(), "rust: tests stripped");
    let php = config.resolve_config_strip("php");
    assert!(php.drop_comments(), "php: comments stripped (cross-lang)");
    assert!(!php.drop_tests(), "php: tests kept");
}

#[test]
fn unknown_language_falls_back_to_cross_language() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    std::fs::write(&path, "strip = [\"imports\"]\n").expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    assert!(config.resolve_config_strip("javascript").drop_imports());
}

#[test]
fn cli_override_beats_per_language_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    std::fs::write(&path, "[languages.rust]\nstrip = []\n").expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    let config_strip = config.resolve_config_strip("rust");
    assert_eq!(config_strip, StripSet::empty(), "config says strip nothing");
    let cli = StripSet::empty().insert(StripSet::COMMENTS);
    let drops = config.resolve_drops("rust", FileStrip::new(Some(cli), StripSet::empty()));
    assert!(drops.drop_comments, "CLI wins over per-lang config");
}

#[test]
fn cli_override_is_global() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    std::fs::write(&path, "[languages.php]\nstrip = []\n").expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    let cli = StripSet::empty().insert(StripSet::IMPORTS);
    let php_drops = config.resolve_drops("php", FileStrip::new(Some(cli), StripSet::empty()));
    assert!(php_drops.drop_imports, "CLI beats per-lang empty");
    let rust_drops = config.resolve_drops("rust", FileStrip::new(Some(cli), StripSet::empty()));
    assert!(rust_drops.drop_imports, "CLI applies globally");
}

#[test]
fn no_cli_strip_falls_through_to_config_layering() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contasty.toml");
    std::fs::write(&path, "strip = [\"comments\"]\n").expect("write");
    let config = Config::load(Some(&path), dir.path(), None);
    // cli == None: the config-layered default for the language is the base.
    let drops = config.resolve_drops("rust", FileStrip::new(None, StripSet::empty()));
    assert!(
        drops.drop_comments,
        "cross-lang config applies when no CLI strip"
    );
    assert!(
        !drops.drop_imports,
        "config kept imports; built-in default not used"
    );
    assert!(!drops.drop_bodies, "config kept bodies");
}

#[test]
fn query_strip_unions_onto_resolved_base() {
    let config = Config::default();
    // No CLI strip, no config: base is built-in (comments, imports, body).
    // Query adds tests; union strips all four.
    let query = StripSet::empty().insert(StripSet::TESTS);
    let drops = config.resolve_drops("rust", FileStrip::new(None, query));
    assert!(drops.drop_comments && drops.drop_imports && drops.drop_bodies);
    assert!(drops.drop_tests, "query unions tests onto the base");
}

#[test]
fn strip_section_round_trips_toml() {
    let toml = "\
strip = [\"comments\", \"imports\"]\n\
[languages.rust]\n\
strip = [\"body\"]\n\
[languages.php]\n\
strip = [\"tests\", \"imports\"]\n";
    let config: Config = toml::from_str(toml).expect("parse");
    let cross = config.strip.expect("cross-lang strip").0;
    assert!(cross.drop_comments());
    assert!(cross.drop_imports());
    assert!(!cross.drop_tests());
    let rust = config.languages.get("rust").expect("rust");
    let rust_set = rust.strip.expect("rust strip").0;
    assert!(rust_set.drop_bodies());
    assert!(!rust_set.drop_comments());
    let php = config.languages.get("php").expect("php");
    let php_set = php.strip.expect("php strip").0;
    assert!(php_set.drop_tests());
    assert!(php_set.drop_imports());
}

#[test]
fn language_block_parses_grammar_and_rule_fields() {
    let toml = "\
[languages.jsonc]\n\
libraryPath = \"grammars/jsonc.so\"\n\
languageSymbol = \"tree_sitter_json\"\n\
extensions = [\"jsonc\"]\n\
rules = \"rules/jsonc.yml\"\n\
extend = \"rules/jsonc-extra.yml\"\n\
strip = [\"tests\"]\n";
    let config: Config = toml::from_str(toml).expect("parse");
    let jsonc = config.languages.get("jsonc").expect("jsonc entry");
    assert!(jsonc.is_dynamic());
    assert_eq!(jsonc.language_symbol.as_deref(), Some("tree_sitter_json"));
    assert_eq!(jsonc.extensions, vec!["jsonc".to_owned()]);
    assert_eq!(jsonc.rules.as_deref(), Some(Path::new("rules/jsonc.yml")));
    assert!(jsonc.strip.expect("strip").0.drop_tests());
    assert!(matches!(
        jsonc.rule_source(),
        Ok(Some(RuleSource::Extend(_)))
    ),);
}

#[test]
fn builtin_language_block_is_not_dynamic() {
    let config: Config =
        toml::from_str("[languages.rust]\nstrip = [\"comments\"]\n").expect("parse");
    let rust = config.languages.get("rust").expect("rust entry");
    assert!(!rust.is_dynamic());
    assert!(matches!(rust.rule_source(), Ok(None)));
}

#[test]
fn both_extend_and_override_is_a_rule_source_error() {
    let config: Config =
        toml::from_str("[languages.rust]\nextend = \"a.yml\"\noverride = \"b.yml\"\n")
            .expect("parse");
    let rust = config.languages.get("rust").expect("rust entry");
    assert!(rust.rule_source().is_err());
}

#[test]
fn parse_list_basic_categories() {
    let set = StripSet::parse_list("comments,imports").expect("parse");
    assert!(set.drop_comments());
    assert!(set.drop_imports());
    assert!(!set.drop_tests());
    assert!(!set.drop_bodies());
}

#[test]
fn parse_list_all_and_none() {
    let all = StripSet::parse_list("all").expect("all");
    assert_eq!(all, StripSet::all());
    let everything = StripSet::parse_list("everything").expect("everything");
    assert_eq!(everything, StripSet::all());
    let none = StripSet::parse_list("none").expect("none");
    assert_eq!(none, StripSet::empty());
}

#[test]
fn parse_list_negation() {
    let set = StripSet::parse_list("all,!body").expect("parse");
    assert!(set.drop_comments());
    assert!(set.drop_imports());
    assert!(set.drop_tests());
    assert!(!set.drop_bodies());
}

#[test]
fn parse_list_none_then_add() {
    let set = StripSet::parse_list("none,body").expect("parse");
    assert!(!set.drop_comments());
    assert!(!set.drop_imports());
    assert!(!set.drop_tests());
    assert!(set.drop_bodies());
}

#[test]
fn parse_list_unknown_category_is_error() {
    assert!(StripSet::parse_list("widgets").is_err());
}

#[test]
fn strip_empty_array_means_strip_nothing() {
    let config: Config = toml::from_str("strip = []\n").expect("parse");
    let set = config.strip.expect("strip present").0;
    assert_eq!(set, StripSet::empty());
}
