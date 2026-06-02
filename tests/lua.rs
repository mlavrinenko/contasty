//! Lua stripping behaviour, driven by `src/lang/rules/lua.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, drop_comments: bool, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.lua")).expect("lua registered");
    lang.strip(
        src,
        Path::new("x.lua"),
        false,
        drop_comments,
        false,
        compact,
    )
    .expect("strip")
}

#[test]
fn lua_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.lua")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/lua/sample.lua");
    let expected = include_str!("fixtures/lua/sample.stripped.lua");
    let out = strip(src, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn keeps_function_bodies() {
    // `{}` is a Lua table, not a valid statement, so bodies are left intact.
    let src = "function add(a, b)\n  return a + b\nend\n";
    let out = strip(src, false, &CompactConfig::default());
    assert!(out.contains("return a + b"), "body dropped: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "-- gone\nlocal x = 1\n";
    let out = strip(src, true, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "local s = \"this string is long enough to truncate\"\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
