//! JSON stripping behaviour, driven by `src/lang/rules/json.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.json")).expect("json registered");
    lang.strip(src, Path::new("x.json"), false, false, false, true, compact)
        .expect("strip")
}

#[test]
fn json_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.json")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/json/sample.json");
    let expected = include_str!("fixtures/json/sample.stripped.json");
    let out = strip(src, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn truncates_long_string_values_keeps_keys() {
    let src = "{ \"a long key name here\": \"this value is long enough to truncate\" }\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, &compact);
    assert!(out.contains("a long key name here"), "key truncated: {out}");
    assert!(!out.contains("long enough"), "value kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "value truncated: {out}");
}

#[test]
fn keeps_numbers_and_structure() {
    let src = "{ \"x\": 1, \"list\": [1, 2, 3] }\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 0,
    };
    let out = strip(src, &compact);
    assert!(
        out.contains("\"list\": [1, 2, 3]"),
        "structure changed: {out}"
    );
}
