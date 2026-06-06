//! CSS stripping behaviour, driven by `src/lang/rules/css.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, drop_comments: bool, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.css")).expect("css registered");
    lang.strip(
        src,
        Path::new("x.css"),
        false,
        drop_comments,
        false,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn css_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.css")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/css/sample.css");
    let expected = include_str!("fixtures/css/sample.stripped.css");
    let out = strip(src, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn truncates_long_strings_keeps_selectors_and_properties() {
    let src = ".btn {\n  color: red;\n  content: \"this content is long enough to truncate\";\n}\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, &compact);
    assert!(out.contains(".btn"), "selector dropped: {out}");
    assert!(out.contains("color: red;"), "property dropped: {out}");
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "/* gone */\n.btn { color: red; }\n";
    let out = strip(src, true, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
    assert!(out.contains(".btn"), "{out}");
}
