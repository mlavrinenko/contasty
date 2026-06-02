//! YAML stripping behaviour, driven by `src/lang/rules/yaml.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, drop_comments: bool, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.yaml")).expect("yaml registered");
    lang.strip(
        src,
        Path::new("x.yaml"),
        false,
        drop_comments,
        false,
        compact,
    )
    .expect("strip")
}

#[test]
fn yaml_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.yaml")).is_some());
    assert!(reg.detect(Path::new("foo.yml")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/yaml/sample.yaml");
    let expected = include_str!("fixtures/yaml/sample.stripped.yaml");
    let out = strip(src, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn truncates_long_quoted_scalars_keeps_plain_keys() {
    let src = "name: keepme\ndesc: \"this scalar is long enough to truncate\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, &compact);
    assert!(
        out.contains("name: keepme"),
        "plain scalar truncated: {out}"
    );
    assert!(!out.contains("long enough"), "value kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "value truncated: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "# gone\nname: keepme\n";
    let out = strip(src, true, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
    assert!(out.contains("name: keepme"), "{out}");
}
