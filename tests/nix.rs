//! Nix stripping behaviour, driven by `src/lang/rules/nix.yml`.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn strip(src: &str, drop_comments: bool, compact: &CompactConfig) -> String {
    let reg = Registry::new().expect("registry init");
    let lang = reg.detect(Path::new("x.nix")).expect("nix registered");
    lang.strip(
        src,
        Path::new("x.nix"),
        false,
        drop_comments,
        false,
        true,
        compact,
    )
    .expect("strip")
}

#[test]
fn nix_extension_is_registered() {
    let reg = Registry::new().expect("registry init");
    assert!(reg.detect(Path::new("foo.nix")).is_some());
}

#[test]
fn fixture_strips_to_snapshot() {
    let src = include_str!("fixtures/nix/sample.nix");
    let expected = include_str!("fixtures/nix/sample.stripped.nix");
    let out = strip(src, true, &CompactConfig::default());
    assert_eq!(out, expected, "stripped output drifted from snapshot");
}

#[test]
fn keeps_bindings() {
    let src = "let\n  add = a: b: a + b;\nin add\n";
    let out = strip(src, false, &CompactConfig::default());
    assert!(out.contains("add = a: b: a + b"), "binding dropped: {out}");
}

#[test]
fn drops_comments_under_flag() {
    let src = "# gone\n{ x = 1; }\n";
    let out = strip(src, true, &CompactConfig::default());
    assert!(!out.contains("gone"), "comment kept: {out}");
}

#[test]
fn truncates_long_strings_above_threshold() {
    let src = "{ s = \"this string is long enough to truncate\"; }\n";
    let compact = CompactConfig {
        elide_min_bytes: 0,
        max_string_bytes: 8,
    };
    let out = strip(src, false, &compact);
    assert!(!out.contains("long enough"), "string kept: {out}");
    assert_eq!(out.matches("[…CTY]").count(), 1, "string truncated: {out}");
}
